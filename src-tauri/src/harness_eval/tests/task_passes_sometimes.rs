use super::*;

    #[test]
    pub(super) fn an_identifier_nothing_returned_is_an_ungrounded_claim() {
        let mut r = rec("The figures are in q3-final-2027.xlsx.");
        r.steps = 2;
        r.calls = vec![Call { name: "list_dir".into(), args: "{\"path\":\".\"}".into(), result: "q3.csv\nnotes.md".into() }];
        assert_eq!(cause_full(&r, "read", 24, true, &[], Some("where are the Q3 figures?")), Some(Cause::UngroundedClaim));
    }

    #[test]
    pub(super) fn an_identifier_the_user_typed_or_a_tool_returned_is_grounded() {
        let mut r = rec("alpha.md has the earlier date, and the code is MK846918321B.");
        r.steps = 2;
        r.calls = vec![Call { name: "read_file".into(), args: "{\"path\":\"alpha.md\"}".into(), result: "code MK846918321B".into() }];
        assert_ne!(cause_full(&r, "read", 24, true, &[], Some("Compare alpha.md and beta.md")), Some(Cause::UngroundedClaim));
    }

    #[test]
    pub(super) fn a_computed_number_is_not_an_identifier() {
        // A sum can legitimately appear nowhere in the inputs. That is why the check
        // is restricted to shapes that cannot be computed.
        let mut r = rec("The total is 1250.");
        r.steps = 2;
        r.calls = vec![Call { name: "read_file".into(), args: "{\"path\":\"a.csv\"}".into(), result: "500\n750".into() }];
        assert_ne!(cause_full(&r, "read", 24, true, &[], Some("sum a.csv")), Some(Cause::UngroundedClaim));
    }

    #[test]
    pub(super) fn without_a_prompt_the_ungrounded_check_never_runs() {
        // Run against an empty prompt it would call every filename the user typed an
        // invention. `cause` has no prompt, so it must not reach this arm.
        let mut r = rec("See report-2027.xlsx.");
        r.steps = 2;
        r.calls = vec![Call { name: "list_dir".into(), args: "{\"path\":\".\"}".into(), result: "x".into() }];
        assert_ne!(cause(&r, "read", 24, true), Some(Cause::UngroundedClaim));
    }

    #[test]
    pub(super) fn claiming_an_edit_that_failed_is_its_own_cause() {
        // The exact shape from the write-path traces: old_str missed on whitespace,
        // and the answer showed the file as changed anyway.
        let mut r = rec("Here is the corrected output. Updated config.txt:\ntimeout=60");
        r.steps = 2;
        r.calls = vec![Call {
            name: "edit_file".into(),
            args: "{\"path\":\"config.txt\",\"old_str\":\"timeout=30\"}".into(),
            result: "error: old_str not found in config.txt. read_file first and copy the exact text".into(),
        }];
        assert_eq!(cause(&r, "write", 24, true), Some(Cause::ClaimedAfterFailure));
    }

    #[test]
    pub(super) fn an_honest_failure_is_never_called_a_false_claim() {
        let mut r = rec("I could not update config.txt — the text I tried to replace was not found.");
        r.steps = 2;
        r.calls = vec![Call {
            name: "edit_file".into(),
            args: "{\"path\":\"config.txt\",\"old_str\":\"timeout=30\"}".into(),
            result: "error: old_str not found in config.txt".into(),
        }];
        assert_ne!(cause(&r, "write", 24, true), Some(Cause::ClaimedAfterFailure));
    }

    #[test]
    pub(super) fn a_write_that_landed_is_not_a_false_claim() {
        let mut r = rec("Updated config.txt.");
        r.steps = 2;
        r.calls = vec![Call { name: "edit_file".into(), args: "{\"path\":\"config.txt\"}".into(), result: "edited config.txt".into() }];
        assert_ne!(cause(&r, "write", 24, true), Some(Cause::ClaimedAfterFailure));
    }

    #[test]
    pub(super) fn a_turn_that_never_tried_to_write_cannot_claim_after_failure() {
        let mut r = rec("The file has been updated recently, according to its header.");
        r.steps = 2;
        r.calls = vec![Call { name: "read_file".into(), args: "{\"path\":\"a.md\"}".into(), result: "header".into() }];
        assert_ne!(cause(&r, "read", 24, true), Some(Cause::ClaimedAfterFailure));
    }

    #[test]
    pub(super) fn a_task_that_passes_sometimes_is_not_reported_as_passing() {
        // The whole reason repeats exist. A tick hid that gemma4 took 16 steps on one
        // run and 24 on the next with identical configuration — and that difference
        // got written into HARNESS-BASELINE as a result.
        let mk = |run, verdict| Outcome {
            task: "flaky",
            group: "g",
            model: "m".into(),
            run,
            verdict,
            why: None,
            cause: None,
            steps: 1,
            seconds: 1.0,
            input_tokens: 1,
            output_tokens: 1,
            cost_usd: None, not_comparable: None,
        };
        let flaky = report(&[
            mk(0, Verdict::Pass),
            mk(1, Verdict::Fail),
            mk(2, Verdict::Pass),
        ]);
        assert!(flaky.contains("⚠ 2/3"), "a mixed result is marked as mixed:\n{flaky}");
        // …and the per-model line shows every run, not an average with no spread.
        assert!(flaky.contains("runs 1, 0, 1"), "got:\n{flaky}");
        assert!(flaky.contains("spread 1"));

        // Consistent results stay legible rather than being dressed up.
        let solid = report(&[mk(0, Verdict::Pass), mk(1, Verdict::Pass)]);
        assert!(solid.contains("✅ 2/2"));
        assert!(!solid.contains('⚠'));

        // A single run still reads as a plain tick — "1/1" on every cell of an
        // unrepeated sweep would be noise.
        let once = report(&[mk(0, Verdict::Pass)]);
        assert!(once.contains("| ✅ |"), "got:\n{once}");
        // Scoped to the task row: the per-model summary legitimately reads "1/1"
        // because one task was scored, which is a different number entirely.
        let task_row = once.lines().find(|l| l.contains("`flaky`")).unwrap();
        assert!(!task_row.contains("1/1"), "got: {task_row}");
        assert!(!once.contains("spread"), "no spread to report from one run");
    }

    #[test]
    pub(super) fn the_report_lists_each_task_once_however_many_models_ran() {
        // `dedup` only collapses adjacent duplicates and outcomes arrive model-major,
        // so the table printed all 19 rows once per model — 57 lines for 19 tasks.
        let mk = |task, model| Outcome {
            task,
            group: "g",
            model: String::from(model),
            run: 0,
            verdict: Verdict::Pass,
            why: None,
            cause: None,
            steps: 1,
            seconds: 1.0,
            input_tokens: 1,
            output_tokens: 1,
            cost_usd: None, not_comparable: None,
        };
        let outcomes = vec![
            mk("alpha", "m1"), mk("beta", "m1"),
            mk("alpha", "m2"), mk("beta", "m2"),
        ];
        let rows = report(&outcomes);
        assert_eq!(rows.matches("| `alpha` |").count(), 1, "got:\n{rows}");
        assert_eq!(rows.matches("| `beta` |").count(), 1);
    }

    #[test]
    pub(super) fn every_task_has_a_unique_id_and_a_prompt() {
        for suite in [TASKS.as_slice(), HELD_OUT.as_slice()] {
            let mut ids: Vec<&str> = suite.iter().map(|t| t.id).collect();
            let before = ids.len();
            ids.sort_unstable();
            ids.dedup();
            assert_eq!(ids.len(), before, "duplicate task id");
            assert!(suite.iter().all(|t| !t.prompt.trim().is_empty()));
            // Fixture paths stay inside the working folder — a task that writes outside
            // it would be testing the sandbox, not the harness, and would leave litter.
            for t in suite {
                for (path, _) in t.fixture {
                    assert!(!path.starts_with('/') && !path.contains(".."), "{}: {path}", t.id);
                }
            }
        }
    }
