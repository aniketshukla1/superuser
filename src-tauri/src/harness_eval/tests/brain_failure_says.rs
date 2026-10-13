use super::*;

    #[test]
    pub(super) fn the_conflict_task_cannot_be_passed_by_hedging() {
        let check = task("prefers_the_newer_fact").check;
        let mut hedged = rec("It was Dana Okoye, and it is now TOKEN.");
        hedged.marker = "TOKEN".into();
        assert!(check(&hedged).is_err(), "naming both must not pass");

        let mut clean = rec("TOKEN is carrying the platform pager.");
        clean.marker = "TOKEN".into();
        assert!(check(&clean).is_ok(), "the current name alone must pass");

        let mut stale = rec("Dana Okoye is carrying the platform pager.");
        stale.marker = "TOKEN".into();
        assert!(check(&stale).is_err(), "the superseded name must not pass");
    }

    #[test]
    pub(super) fn the_invented_memory_task_refuses_a_figure_and_accepts_an_admission() {
        let check = task("does_not_invent_a_memory").check;
        assert!(check(&rec("I don't have anything about the Q3 budget.")).is_ok());
        assert!(check(&rec("The Q3 infrastructure budget is $1.2M.")).is_err(), "a figure must fail");
        // The trap this guards: an answer that both admits and answers. "not" alone
        // would let it through, so the figure check has to be independent.
        assert!(
            check(&rec("There is not much detail, but the Q3 budget is $900k.")).is_err(),
            "admitting and then inventing must still fail"
        );
    }

    #[test]
    pub(super) fn the_preference_task_counts_bullets_and_not_words() {
        let check = task("applies_a_stated_preference").check;
        assert!(check(&rec("- one\n- two\n- three")).is_ok());
        assert!(check(&rec("- one\n- two")).is_err(), "two bullets is not three");
        assert!(check(&rec("- one\n- two\n- three\n- four")).is_err(), "four bullets is not three");
        // Prose that happens to mention three risks is not the format that was asked
        // for. The rule is the thing being tested, not the content.
        assert!(check(&rec("The three risks are fatigue, thin cover and slow rollback.")).is_err());
    }

    #[test]
    pub(super) fn every_brain_task_seeds_a_vault() {
        // Otherwise it is not a brain task, it is a general-knowledge question filed
        // under the wrong group — and the group total would say the brain works.
        for t in TASKS.iter().chain(HELD_OUT.iter()).filter(|t| t.group == "brain") {
            assert!(!t.vault.is_empty(), "{}: group is brain and it seeds no memories", t.id);
        }
    }

    #[test]
    pub(super) fn the_apps_own_apology_is_never_an_answer() {
        // A false pass is worse than a false failure: a red cell gets investigated
        // and a green one gets believed. This one was real — `gemma4:12b` returned
        // empty replies, and `does_not_invent_a_memory` passed on the apology
        // because it contains the word "couldn't".
        let mut r = rec("I couldn't produce an answer for that — the model returned an empty reply. Try rephrasing.");
        r.steps = 2;
        assert!(universal_check(&r, 24).is_err(), "the apology must not be gradeable");
        // And the honesty check that was fooled agrees, now that it never sees it.
        let honest = rec("I don't have anything in memory about the Q3 budget.");
        assert!(universal_check(&honest, 24).is_ok(), "a real admission still passes");
    }

    #[test]
    pub(super) fn a_brain_failure_says_whether_the_retriever_missed() {
        // The distinction the board could not draw. Both of these are a red cell in
        // the brain group and only one of them is ours.
        let seeded: &[Fixture] = &[("m.md", "---\ntitle: \"Platform on-call rotation\"\ncategory: \"Eng\"\n---\n\nbody\n")];

        // Recall came back without the memory the task planted: the retriever missed.
        let mut missed = rec("I don't know who is on call.");
        missed.steps = 2;
        missed.calls = vec![Call {
            name: "recall_memory".into(),
            args: "{\"query\":\"platform pager\"}".into(),
            result: "no relevant memories found".into(),
        }];
        assert_eq!(
            cause_with_vault(&missed, "brain", 24, true, seeded),
            Some(Cause::RetrievalMiss),
            "the model was never handed the memory"
        );

        // Recall returned it and the answer is still wrong: that is the model.
        let mut ignored = rec("Dana Okoye is on call.");
        ignored.steps = 2;
        ignored.calls = vec![Call {
            name: "recall_memory".into(),
            args: "{\"query\":\"platform pager\"}".into(),
            result: "### Platform on-call rotation (github-9440.md) — dated 2026-08-20\nbody".into(),
        }];
        assert_eq!(
            cause_with_vault(&ignored, "brain", 24, true, seeded),
            Some(Cause::Model),
            "it was handed the memory and got it wrong anyway"
        );
    }

    #[test]
    pub(super) fn retrieval_attribution_never_fires_outside_the_brain_group() {
        // Every other task seeds no vault, so this arm must be unreachable for them
        // — otherwise a read task that happens to call `recall_memory` would be
        // blamed on the retriever.
        let mut r = rec("wrong answer");
        r.steps = 2;
        r.calls = vec![Call { name: "read_file".into(), args: "{\"path\":\"a.md\"}".into(), result: "x".into() }];
        assert_eq!(cause_with_vault(&r, "read", 24, true, &[]), Some(Cause::Model));
        assert_eq!(cause(&r, "read", 24, true), Some(Cause::Model), "the old signature is unchanged");
    }

    #[test]
    pub(super) fn a_fixture_with_no_parseable_title_is_not_treated_as_a_miss() {
        // Guessing a title wrong would attribute a model failure to the retriever,
        // which is the confusion this whole arm exists to end.
        let untitled: &[Fixture] = &[("m.md", "no frontmatter here\n")];
        let mut r = rec("wrong");
        r.steps = 2;
        r.calls = vec![Call { name: "recall_memory".into(), args: "{\"query\":\"x\"}".into(), result: "nothing".into() }];
        assert_eq!(cause_with_vault(&r, "brain", 24, true, untitled), Some(Cause::Model));
    }

    #[test]
    pub(super) fn a_call_to_a_tool_that_does_not_exist_is_its_own_cause() {
        let mut r = rec("done");
        r.steps = 2;
        r.calls = vec![Call { name: "read_spreadsheet".into(), args: "{\"p\":\"a\"}".into(), result: "unknown tool".into() }];
        assert_eq!(cause(&r, "read", 24, true), Some(Cause::ToolHallucination));
    }

    #[test]
    pub(super) fn a_runtime_synthesized_tool_is_real() {
        // `find_tools` and `load_skill` are minted by the loop rather than declared
        // in the catalogue, and calling them a hallucination would be a confident
        // wrong answer in the one column that exists to avoid those.
        for name in ["find_tools", "load_skill", "recall_output", "read_file"] {
            let mut r = rec("done");
            r.steps = 2;
            r.calls = vec![Call { name: name.into(), args: "{\"q\":\"x\"}".into(), result: "ok".into() }];
            assert_ne!(cause(&r, "read", 24, true), Some(Cause::ToolHallucination), "{name} is real");
        }
    }

    #[test]
    pub(super) fn an_mcp_tool_is_never_called_invented() {
        // Server tools are real and deliberately outside the catalogue. Reporting
        // them as hallucinations would libel every MCP user.
        let mut r = rec("done");
        r.steps = 2;
        r.calls = vec![Call { name: "notion__search".into(), args: "{\"q\":\"x\"}".into(), result: "error: rate limited".into() }];
        assert_ne!(cause(&r, "read", 24, true), Some(Cause::ToolHallucination));
    }

    #[test]
    pub(super) fn three_failures_of_one_tool_on_three_arguments_is_a_wall_not_a_search() {
        let mk = |a: &str| Call { name: "read_file".into(), args: a.into(), result: "error: no such path".into() };
        let mut r = rec("I could not find it");
        r.steps = 4;
        r.calls = vec![mk("{\"path\":\"a\"}"), mk("{\"path\":\"b\"}"), mk("{\"path\":\"c\"}")];
        assert_eq!(cause(&r, "read", 24, true), Some(Cause::ReentrantFailure));
    }

    #[test]
    pub(super) fn a_failure_between_successes_is_a_bad_day_not_a_loop() {
        let fail = |a: &str| Call { name: "read_file".into(), args: a.into(), result: "error: no such path".into() };
        let ok = Call { name: "read_file".into(), args: "{\"path\":\"real\"}".into(), result: "contents".into() };
        let mut r = rec("done");
        r.steps = 4;
        r.calls = vec![fail("{\"path\":\"a\"}"), ok.clone(), fail("{\"path\":\"b\"}"), ok];
        assert_ne!(cause(&r, "read", 24, true), Some(Cause::ReentrantFailure));
    }

    #[test]
    pub(super) fn identical_repeated_arguments_stay_with_the_repetition_gate() {
        // `gates::repetition` owns that case and reports it better, so this arm must
        // not steal it.
        let same = Call { name: "read_file".into(), args: "{\"path\":\"a\"}".into(), result: "error: no such path".into() };
        let mut r = rec("gave up");
        r.steps = 4;
        r.calls = vec![same.clone(), same.clone(), same];
        assert_ne!(cause(&r, "read", 24, true), Some(Cause::ReentrantFailure));
    }
