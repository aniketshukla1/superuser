use super::*;

    pub(super) fn spec(name: &str, capability: &str, pad: usize) -> ToolSpec {
        ToolSpec {
            name: name.into(),
            description: "x".repeat(pad),
            schema: json!({ "type": "object", "properties": {} }),
            risk: RiskClass::Read,
            capability: capability.into(),
        }
    }

    pub(super) fn profile(tier: Tier, tools: ToolSupport) -> ModelProfile {
        let mut p = assumed("test");
        p.tier = tier;
        p.tools = tools;
        p
    }

    #[test]
    pub(super) fn the_control_condition_is_wider_than_every_tier() {
        // The scoreboard's "Phase 5 off" arm. If this were not strictly the widest
        // shape, the A/B would be measuring something other than tier shaping — so it
        // is asserted rather than assumed.
        let s = Settings::default();
        let off = LoopShape::unshaped(&s);
        assert_eq!(off.tool_budget, None, "nothing withheld");
        assert!(off.skills_in_prompt);
        assert!(off.parallel_tools, "risk class alone, as it was before Phase 5");
        for tier in [Tier::Small, Tier::Mid, Tier::Frontier] {
            let on = LoopShape::for_model(&profile(tier, ToolSupport::Sequential), &s);
            assert!(on.max_steps <= off.max_steps, "{tier:?} steps");
        }
        // **No tier withholds tools.** Each arrived there by its own measurement and
        // neither is a default: Small was narrowed and scored worse for it; Mid was
        // narrowed and paid a ~30s `find_tools` round trip for it; Frontier never was.
        // Asserted together so that re-narrowing any of them has to be a deliberate
        // edit here, with a number behind it, rather than something that drifts back in.
        for tier in [Tier::Small, Tier::Mid, Tier::Frontier] {
            assert_eq!(
                LoopShape::for_model(&profile(tier, ToolSupport::Sequential), &s).tool_budget,
                None,
                "{tier:?} is unnarrowed — see the tier arms in `for_model` for what was measured"
            );
        }
    }

    #[test]
    pub(super) fn narrowing_keeps_every_tool_the_regressed_tasks_needed() {
        // The strangest thing the scoreboard found, pinned so it cannot be explained
        // away later. Narrowing costs `llama3.2:3b` four tasks — `one_word`,
        // `read_named_file`, `counts_what_is_there`, `does_not_reread` — and **not one
        // of them uses a withheld tool**. `one_word` calls nothing at all; the other
        // three need only `read_file`, `list_dir` and `run_shell`, and this asserts all
        // three survive the Small budget on the persona the suite actually runs.
        //
        // So the regression is not "the tool it needed was taken away". A shorter list
        // makes this model worse at tasks using only the tools it still has, and no
        // mechanism here explains that.
        let ctx = crate::core::agent::AgentContext {
            workspace: Some(std::path::PathBuf::from("/tmp/ws")),
            roots: vec![std::path::PathBuf::from("/tmp/ws")],
            has_executor: true,
            has_todo: true,
        };
        // The Main Assistant's declared capabilities — the persona the scoreboard runs.
        let caps: Vec<String> = [
            "recall_memory", "knowledge", "web", "files", "shell", "todo", "connectors",
            "schedule", "subagent",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let specs = crate::core::agent::expand(&caps, &ctx);
        let shape = LoopShape::for_model(&profile(Tier::Small, ToolSupport::Sequential), &Settings::default());
        let (sent, _held) = shape.split_tools(&specs);
        let names: Vec<&str> = sent.iter().map(|t| t.name.as_str()).collect();
        for needed in ["read_file", "list_dir", "run_shell", "write_file", "edit_file", "ask_user"] {
            assert!(names.contains(&needed), "Small dropped {needed}; sent {names:?}");
        }
    }

    #[test]
    pub(super) fn a_frontier_model_is_sent_the_whole_catalogue() {
        let shape = LoopShape::for_model(
            &profile(Tier::Frontier, ToolSupport::Parallel),
            &Settings::default(),
        );
        assert_eq!(shape.tool_budget, None);
        let specs: Vec<ToolSpec> = (0..40).map(|i| spec(&format!("t{i}"), "misc", 400)).collect();
        let (sent, held) = shape.split_tools(&specs);
        assert_eq!(sent.len(), 40);
        assert!(held.is_empty());
    }

    /// A shape with an explicit budget, for testing the budgeting itself.
    ///
    /// These used to reach the mechanism through `for_model(Small)`, which coupled them
    /// to a tier policy — and when the measurements changed that policy, three tests
    /// about arithmetic broke for a reason that had nothing to do with arithmetic.
    pub(super) fn budgeted(tokens: usize) -> LoopShape {
        LoopShape { tool_budget: Some(tokens), ..LoopShape::unshaped(&Settings::default()) }
    }

    #[test]
    pub(super) fn an_empty_reply_after_no_work_does_not_claim_success() {
        // "Done." reads to the user exactly like success. It is a fair summary of a
        // turn that ran tools and had nothing left to add, and a lie about a turn
        // where nothing happened at all — which is how three scoreboard tasks scored
        // a polite "Done." against prompts they never addressed.
        assert_eq!(empty_answer(true), "Done.");
        let nothing = empty_answer(false);
        assert!(nothing.contains("empty reply"), "it says what went wrong: {nothing}");
        assert_ne!(nothing, "Done.");
        // And it tells the user what to do next rather than only what failed.
        assert!(nothing.contains("rephrasing") || nothing.contains("switch models"));
    }

    #[test]
    pub(super) fn the_nudge_asks_for_words_rather_than_repeating_the_question() {
        // The model has the question already; what it lacks is the instruction to
        // answer in prose. Repeating the prompt would invite the same empty reply.
        assert!(EMPTY_REPLY_NUDGE.contains("empty"));
        assert!(EMPTY_REPLY_NUDGE.contains("in words"));
        assert!(EMPTY_REPLY_NUDGE.contains("blocking"), "it offers a way to say why");
    }

    #[test]
    pub(super) fn the_step_budget_warns_early_enough_to_be_steerable() {
        // The bug this replaces: a fixed three-remaining threshold fires at step 22 of
        // 24. Graded, that turned gemma4's silent 24-step loop into a 23-step loop
        // that answered — nothing became something, and the eighteen wasted minutes
        // stayed wasted. A budget is only steerable while enough of it is left.
        assert_eq!(step_notice(0, 24), None, "silent while there is room");
        assert_eq!(step_notice(11, 24), None);
        // Halfway: 12 remain, and there is still a turn's worth of room to change tack.
        assert!(step_notice(12, 24).unwrap().contains("Half"), "fires at the midpoint");
        assert_eq!(step_notice(13, 24), None, "once, not every step after");
        // A quarter left, and the wording gets firmer.
        assert!(step_notice(18, 24).unwrap().contains("Stop gathering"));
        assert_eq!(step_notice(19, 24), None);
        // Exactly three notices in a whole turn, whatever the budget.
        let fired = (0..24).filter(|u| step_notice(*u, 24).is_some()).count();
        assert_eq!(fired, 3, "half, quarter, final — and nothing else");
    }

    #[test]
    pub(super) fn the_milestones_track_the_budget_rather_than_a_fixed_count() {
        // The whole point of the change: the same three moments, wherever they fall.
        for max in [8, 12, 24, 40] {
            let fired: Vec<usize> = (0..max).filter(|u| step_notice(*u, max).is_some()).collect();
            assert_eq!(fired.len(), 3, "max={max} fired at {fired:?}");
            // …and they arrive in order, the first no later than halfway through.
            assert!(fired[0] <= max / 2, "max={max}: first notice at {} is too late", fired[0]);
            assert!(fired[2] == max - 1, "max={max}: the last one is the final step");
        }
    }

    #[test]
    pub(super) fn a_budget_too_small_to_have_thirds_collapses_sensibly() {
        // `max_steps` floors at 4 for a Small tier and a subagent gets
        // `(parent / 2).max(4)`, so tiny budgets are real. The fractions coincide and
        // the more urgent wording has to be the one that survives.
        let fired: Vec<usize> = (0..4).filter(|u| step_notice(*u, 4).is_some()).collect();
        assert!(!fired.is_empty(), "a short turn still gets told");
        assert!(step_notice(3, 4).unwrap().contains("final step"));
        // Never a notice claiming zero or negative room.
        for max in 1..=6 {
            for used in 0..=max + 2 {
                if let Some(n) = step_notice(used, max) {
                    assert!(!n.contains(" 0 "), "max={max} used={used}: {n}");
                }
            }
        }
    }
