//! §4: selecting by relevance instead of by a fixed order

use super::*;

    pub(super) fn rel(pairs: &[(&str, f32)]) -> std::collections::BTreeMap<String, f32> {
        pairs.iter().map(|(n, s)| (n.to_string(), *s)).collect()
    }

    /// Equal-cost tools, so a budget counts them rather than their descriptions.
    pub(super) fn tools(names: &[&str]) -> Vec<ToolSpec> {
        names.iter().map(|n| spec(n, "misc", 400)).collect()
    }

    #[test]
    pub(super) fn the_sweep_budget_can_only_narrow() {
        // The mirror of the FULL_SHAPE guard. A stray environment variable must not
        // be able to hand a model more tools than its tier allows, however it is set.
        let shape = LoopShape { tool_budget: Some(800), max_steps: 12, skills_in_prompt: true, parallel_tools: false };
        // Wider than the tier: the tier wins.
        assert_eq!(Some(800usize).min(Some(2000)), Some(800));
        // Narrower than the tier: the override wins.
        assert_eq!(shape.tool_budget.map(|h| h.min(400)), Some(400));
    }

    #[test]
    pub(super) fn no_budget_means_the_retrieval_flag_changes_nothing() {
        // The reason `HARNESS_EVAL_TOOL_BUDGET` had to exist. With no budget both
        // splits return the whole catalogue, so an ablation between them compares an
        // arm with itself — and a table saying "no difference" would be a statement
        // about a code path that never ran.
        let specs = vec![spec("alpha", "read", 100), spec("beta", "read", 100), spec("gamma", "read", 100)];
        let unbudgeted = LoopShape { tool_budget: None, max_steps: 12, skills_in_prompt: true, parallel_tools: false };
        let (fixed, held_a) = unbudgeted.split_tools(&specs);
        let (ranked, held_b) = unbudgeted.split_tools_retrieved(&specs, &rel(&[("gamma", 0.99)]));
        assert_eq!(fixed.len(), 3);
        assert_eq!(ranked.len(), 3);
        assert!(held_a.is_empty() && held_b.is_empty());
        let names = |v: &Vec<ToolSpec>| v.iter().map(|t| t.name.clone()).collect::<Vec<_>>();
        assert_eq!(names(&fixed), names(&ranked), "with no budget the two arms are the same arm");
    }

    #[test]
    pub(super) fn the_always_sent_set_goes_first_whatever_the_query_scored() {
        // These four are what a turn cannot proceed without. A query about
        // spreadsheets must not cost the model its ability to read a file.
        let specs = tools(&["sheet_new", "read_file", "write_file", "run_shell", "find_tools"]);
        let shape = budgeted(100_000);
        let scores = rel(&[("sheet_new", 0.99), ("read_file", 0.01), ("write_file", 0.01),
                           ("run_shell", 0.01), ("find_tools", 0.01)]);
        let (sent, _held) = shape.split_tools_retrieved(&specs, &scores);
        for must in LoopShape::ALWAYS_SENT {
            assert!(sent.iter().any(|t| t.name == *must), "{must} should always be sent");
        }
    }

    #[test]
    pub(super) fn past_the_always_sent_set_the_query_decides() {
        let specs = tools(&["read_file", "write_file", "run_shell", "find_tools", "alpha", "beta"]);
        let shape = budgeted(schema_tokens(&specs[0]) * 5);
        let (sent, held) = shape.split_tools_retrieved(&specs, &rel(&[("alpha", 0.1), ("beta", 0.9)]));
        assert!(sent.iter().any(|t| t.name == "beta"), "the relevant one is sent");
        assert!(held.iter().any(|t| t.name == "alpha"), "the irrelevant one is held");
    }

    #[test]
    pub(super) fn no_scores_at_all_still_produces_a_working_wire() {
        // The embedder being unavailable must degrade to a worse ranking, never to
        // a turn with no tools on it.
        let specs = tools(&["read_file", "write_file", "run_shell", "find_tools", "alpha"]);
        let shape = budgeted(100_000);
        let (sent, _) = shape.split_tools_retrieved(&specs, &Default::default());
        assert_eq!(sent.len(), specs.len(), "everything fits, so everything goes");
        for must in LoopShape::ALWAYS_SENT {
            assert!(sent.iter().any(|t| t.name == *must));
        }
    }

    #[test]
    pub(super) fn an_unbudgeted_tier_sends_everything_as_it_always_did() {
        let specs = tools(&["read_file", "alpha"]);
        let shape = LoopShape { tool_budget: None, ..LoopShape::unshaped(&Settings::default()) };
        let (sent, held) = shape.split_tools_retrieved(&specs, &rel(&[("alpha", 0.9)]));
        assert_eq!(sent.len(), 2);
        assert!(held.is_empty());
    }

    #[test]
    pub(super) fn equal_scores_break_on_catalogue_order_so_a_run_reproduces() {
        let specs = tools(&["read_file", "write_file", "run_shell", "find_tools", "alpha", "beta"]);
        let shape = budgeted(schema_tokens(&specs[0]) * 5);
        let scores = rel(&[("alpha", 0.5), ("beta", 0.5)]);
        let (a, _) = shape.split_tools_retrieved(&specs, &scores);
        let (b, _) = shape.split_tools_retrieved(&specs, &scores);
        let names = |v: &[ToolSpec]| v.iter().map(|t| t.name.clone()).collect::<Vec<_>>();
        assert_eq!(names(&a), names(&b));
    }

    #[test]
    pub(super) fn the_returned_order_is_the_catalogues_not_the_ranking() {
        // Same guarantee the static split makes: nothing downstream should depend
        // on the order the budget happened to be computed in.
        let specs = tools(&["alpha", "read_file", "beta", "run_shell", "write_file", "find_tools"]);
        let shape = budgeted(100_000);
        let (sent, _) = shape.split_tools_retrieved(&specs, &rel(&[("beta", 0.9), ("alpha", 0.8)]));
        let got: Vec<&str> = sent.iter().map(|t| t.name.as_str()).collect();
        let want: Vec<&str> = specs.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(got, want);
    }
