use super::*;

    /// The two splits may not share a task id.
    ///
    /// A shared id would collide in the report — one row, two different tasks — and
    /// would quietly defeat the point of the split: the held-out number would be
    /// partly a re-run of something already tuned against.
    #[test]
    pub(super) fn the_splits_are_disjoint() {
        for h in HELD_OUT.iter() {
            assert!(
                !TASKS.iter().any(|t| t.id == h.id),
                "{} appears in both splits",
                h.id
            );
        }
    }

    /// Whatever the feedback set exercises, the held-out set exercises too.
    ///
    /// Without this the split silently narrows: a group dropped from `HELD_OUT` is a
    /// capability nobody is measuring generalization on, and the total still looks
    /// like a score.
    #[test]
    pub(super) fn the_held_out_split_covers_every_group() {
        for t in TASKS.iter() {
            assert!(
                HELD_OUT.iter().any(|h| h.group == t.group),
                "group {:?} has no held-out counterpart",
                t.group
            );
        }
    }

    #[test]
    pub(super) fn a_correct_answer_passes_and_a_near_miss_does_not() {
        let t = task("one_word");
        assert!((t.check)(&rec("pong")).is_ok());
        assert!((t.check)(&rec("  Pong. ")).is_ok(), "punctuation and case are not the test");
        assert!((t.check)(&rec("The answer is pong")).is_err(), "'exactly one word' means it");
    }

    #[test]
    pub(super) fn inventing_the_contents_of_a_missing_file_fails() {
        // The most important check in the file: a confident wrong answer is worse
        // than an error, and every other check would pass this one happily.
        let t = task("admits_a_missing_file");
        assert!((t.check)(&rec("missing.md does not exist in this folder.")).is_ok());
        assert!((t.check)(&rec("It says the code name is BLUEJAY.")).is_err());
    }

    #[test]
    pub(super) fn reading_five_files_to_find_one_marker_is_a_failure_even_when_the_answer_is_right() {
        let t = task("grep_before_reading_everything");
        let mut r = rec("four.md");
        r.calls = (0..5).map(|_| Call { name: "read_file".into(), ..Default::default() }).collect();
        assert!((t.check)(&r).is_err(), "brute force is the thing being measured");

        r.calls = vec![
            Call { name: "grep".into(), ..Default::default() },
            Call { name: "read_file".into(), ..Default::default() },
        ];
        assert!((t.check)(&r).is_ok());

        // Right process, wrong answer, still a failure — the process is not the point
        // on its own.
        let mut wrong = rec("two.md");
        wrong.calls = r.calls.clone();
        assert!((t.check)(&wrong).is_err());
    }

    #[test]
    pub(super) fn a_whole_file_rewrite_that_loses_the_other_lines_fails() {
        let t = task("edit_without_clobbering");
        let mut r = rec("done");
        r.files.insert("config.txt".into(), "retries = 5\ntimeout = 60\nverbose = true\n".into());
        assert!((t.check)(&r).is_ok());
        r.files.insert("config.txt".into(), "timeout = 60\n".into());
        assert!((t.check)(&r).is_err(), "clobbering the rest of the file is the failure");
    }

    #[test]
    pub(super) fn editing_the_wrong_one_of_two_identical_lines_fails() {
        let t = task("edit_the_right_occurrence");
        let mut r = rec("done");
        r.files.insert(
            "app.conf".into(),
            "api   host=localhost port=8080\nmetrics host=localhost port=9000\n".into(),
        );
        assert!((t.check)(&r).is_ok());
        r.files.insert(
            "app.conf".into(),
            "api   host=localhost port=9000\nmetrics host=localhost port=9000\n".into(),
        );
        assert!((t.check)(&r).is_err());
    }

    #[test]
    pub(super) fn the_universal_checks_catch_what_no_task_would() {
        // Right answer, absurd cost.
        let mut r = rec("pong");
        r.steps = 40;
        assert!(universal_check(&r, 24).is_err());

        // The live `llama3.2:3b` failure: the tool call arrives as prose, the turn
        // "succeeds", and the answer is machinery. Invisible to every task check.
        let r = rec("{\"name\":\"read_file\",\"parameters\":{\"path\":\"alpha.md\"}}");
        assert!(universal_check(&r, 24).is_err());

        assert!(universal_check(&rec("Reached the step limit."), 24).is_err());
        assert!(universal_check(&rec("  "), 24).is_err());
        let mut failed = rec("anything");
        failed.error = Some("model error 400".into());
        assert!(universal_check(&failed, 24).is_err());
    }

    #[test]
    pub(super) fn a_task_check_never_runs_on_a_turn_that_failed() {
        // Otherwise a 400 shows up as "wrong answer", which sends the next person
        // looking at the prompt instead of at the provider.
        let mut r = rec("pong");
        r.error = Some("529 overloaded".into());
        let o = score(task("one_word"), "m", &r, 24);
        assert_eq!(o.verdict, Verdict::Fail);
        assert!(o.why.unwrap().contains("529"), "the real cause survives");
    }

    #[test]
    pub(super) fn the_report_puts_a_dash_where_a_model_never_ran_a_task() {
        // A blank and a failure are different facts — the same rule the rest of this
        // project follows for cost and cache counters.
        let outcomes = vec![
            Outcome { task: "one_word", group: "instruction", model: "a".into(), run: 0, verdict: Verdict::Pass, why: None, cause: None, steps: 1, seconds: 1.0, input_tokens: 10, output_tokens: 1, cost_usd: None, not_comparable: None },
            Outcome { task: "json_only", group: "instruction", model: "b".into(), run: 0, verdict: Verdict::Fail, why: Some("nope".into()), cause: Some(Cause::Model), steps: 2, seconds: 2.0, input_tokens: 20, output_tokens: 2, cost_usd: None, not_comparable: None },
        ];
        let t = report(&outcomes);
        assert!(t.contains('–'), "missing cell is a dash, not a cross");
        assert!(t.contains("nope"), "the failure reason is named");
        assert!(t.contains('—'), "unknown cost is not $0.00");
    }

    #[test]
    pub(super) fn a_run_the_driver_could_not_measure_is_not_a_failure() {
        // Measured live: `gemma4:12b` spent 108 seconds on one step of a six-step
        // task and blew the driver's own 600-second deadline. Scored as a failure,
        // that reports the harness's timeout as the model's ability — and it would
        // have gone into HARNESS-BASELINE as a fact about gemma4.
        let mut r = rec("");
        r.unreachable = Some("no answer: operation timed out".into());
        let o = score(task("one_word"), "gemma4:12b", &r, 24);
        assert_eq!(o.verdict, Verdict::NoResult);

        // A blank in the table, out of the denominator, and named separately from
        // the failures so nobody has to guess which kind it was.
        let t = report(&[o]);
        assert!(t.contains('–'));
        assert!(t.contains("0/0"), "not counted against the model: {t}");
        assert!(t.contains("Not measured"));
        assert!(!t.contains("**Failures**"));
    }

    #[test]
    pub(super) fn an_error_the_app_returned_is_still_a_failure() {
        // The other side of the same distinction: a provider 400 or a refused gate
        // means the turn ran and did not work. That IS the harness's score.
        let mut r = rec("");
        r.error = Some("model error 400: does not support thinking".into());
        assert_eq!(score(task("one_word"), "m", &r, 24).verdict, Verdict::Fail);
    }

    #[test]
    pub(super) fn a_partial_cost_total_reads_as_unknown() {
        // One unpriced model in the run must not silently make the total look like
        // the whole bill — `core::pricing` keeps free and unknown apart and so does
        // this.
        let outcomes = vec![
            Outcome { task: "a", group: "g", model: "m".into(), run: 0, verdict: Verdict::Pass, why: None, cause: None, steps: 1, seconds: 1.0, input_tokens: 1, output_tokens: 1, cost_usd: Some(0.01), not_comparable: None },
            Outcome { task: "b", group: "g", model: "m".into(), run: 0, verdict: Verdict::Pass, why: None, cause: None, steps: 1, seconds: 1.0, input_tokens: 1, output_tokens: 1, cost_usd: None, not_comparable: None },
        ];
        assert!(report(&outcomes).contains('—'));
    }
