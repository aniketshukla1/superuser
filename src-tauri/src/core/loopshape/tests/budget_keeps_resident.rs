use super::*;

    #[test]
    pub(super) fn the_final_step_is_told_it_is_final_and_gets_no_tools() {
        // The measured failure: gemma4 spent 24 steps and 27 minutes and returned
        // "Reached the step limit." — nothing. A warning alone would be a hint the
        // model can ignore; withholding the tools is what makes it true, and turns
        // "ran out" into "answered with what it had".
        let last = step_notice(23, 24).unwrap();
        assert!(last.contains("final step"), "got: {last}");
        assert!(last.contains("could not finish"), "it has to say what it missed");
        assert!(is_final_step(23, 24));
        assert!(!is_final_step(22, 24));
    }

    #[test]
    pub(super) fn a_budget_already_spent_says_nothing_to_nobody() {
        // Past the end the loop has already stopped. Emitting a warning here would
        // append a message to a turn that is over.
        assert_eq!(step_notice(24, 24), None);
        assert_eq!(step_notice(99, 24), None);
        // …but "past the end" still counts as final, so no caller can offer tools on it.
        assert!(is_final_step(24, 24));
        assert!(is_final_step(99, 24));
    }

    #[test]
    pub(super) fn a_one_step_budget_is_final_from_the_start() {
        // Subagents get (parent / 2).max(4), and a user can lower the setting. A turn
        // with a single step must answer on it rather than spending it on a tool call
        // whose result nothing will ever read.
        assert!(is_final_step(0, 1));
        assert!(step_notice(0, 1).unwrap().contains("final step"));
    }

    #[test]
    pub(super) fn a_budget_keeps_the_resident_tools_and_holds_the_rest() {
        // The point of ranking before budgeting: `read_file` must survive whatever
        // else the persona declares, and a specialised tool must not displace it
        // merely by appearing first in the catalogue.
        let shape = budgeted(1_400);
        // Padded past the budget on purpose: the question is which tools survive when
        // not all of them can, and the real catalogue is measured separately below.
        let specs = vec![
            spec("sheet_write", "sheets", 6_000),
            spec("board_add", "board", 6_000),
            spec("read_file", "files", 200),
            spec("run_shell", "shell", 200),
        ];
        let (sent, held) = shape.split_tools(&specs);
        let names: Vec<&str> = sent.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"read_file") && names.contains(&"run_shell"));
        assert!(held.iter().any(|t| t.name == "sheet_write"));
        assert!(held.iter().any(|t| t.name == "board_add"));
    }

    #[test]
    pub(super) fn the_sent_set_comes_back_in_the_callers_order() {
        // Ranking is for spending the budget, not for the wire. Anything downstream
        // that depends on order — the cache prefix, most of all — must see the order
        // capability expansion produced.
        let shape = budgeted(1_400);
        let specs = vec![
            spec("board_add", "board", 100),
            spec("read_file", "files", 100),
            spec("grep", "search", 100),
        ];
        let (sent, _) = shape.split_tools(&specs);
        let names: Vec<&str> = sent.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, vec!["board_add", "read_file", "grep"]);
    }

    #[test]
    pub(super) fn the_budget_stops_at_the_first_tool_that_does_not_fit() {
        // Not "keep packing with whatever is small enough". Skipping an expensive tool
        // to fit a cheap one behind it would make the sent set depend on schema
        // length instead of on the priority order, which is the only policy here.
        let shape = budgeted(1_400);
        let specs = vec![
            spec("read_file", "files", 100),
            spec("huge", "misc", 40_000),
            spec("tiny", "misc", 10),
        ];
        let (sent, held) = shape.split_tools(&specs);
        assert_eq!(sent.iter().map(|t| t.name.as_str()).collect::<Vec<_>>(), vec!["read_file"]);
        assert_eq!(held.len(), 2, "tiny stays held even though it would fit");
    }

    #[test]
    pub(super) fn a_narrowed_loop_takes_fewer_steps_but_never_more_than_asked_for() {
        let mut s = Settings::default();
        s.agent_max_steps = 24;
        assert_eq!(LoopShape::for_model(&profile(Tier::Small, ToolSupport::Sequential), &s).max_steps, 12);
        assert_eq!(LoopShape::for_model(&profile(Tier::Mid, ToolSupport::Sequential), &s).max_steps, 24);
        // A user who lowered the setting is not overridden upward by a tier.
        s.agent_max_steps = 6;
        assert_eq!(LoopShape::for_model(&profile(Tier::Small, ToolSupport::Sequential), &s).max_steps, 6);
    }

    #[test]
    pub(super) fn parallel_tools_wait_for_a_model_that_says_it_can() {
        // Phase 2 measures this on the OpenAI-compatible path and Ollama never claims
        // it. Emitting several calls to a model that answers one is a step whose
        // later calls are silently dropped.
        assert!(!LoopShape::for_model(&profile(Tier::Frontier, ToolSupport::Sequential), &Settings::default()).parallel_tools);
        assert!(LoopShape::for_model(&profile(Tier::Frontier, ToolSupport::Parallel), &Settings::default()).parallel_tools);
        assert!(LoopShape::for_model(&profile(Tier::Small, ToolSupport::Parallel), &Settings::default()).parallel_tools);
    }

    #[test]
    pub(super) fn a_withheld_tool_can_be_asked_for_by_what_it_does() {
        // Narrowing without this is a capability cut. Anthropic has tool search
        // natively; Ollama and the OpenAI-compatible providers have nothing, so the
        // same idea is built here.
        let held = vec![
            spec("sheet_read", "sheets", 10),
            spec("board_add", "board", 10),
            spec("chart", "files", 10),
        ];
        let (reply, revealed) = reveal(&held, "read the spreadsheet numbers");
        assert!(revealed.iter().any(|t| t.name == "sheet_read"), "got: {reply}");
        assert!(reply.contains("sheet_read"));
    }

    #[test]
    pub(super) fn a_query_in_the_gerund_still_finds_the_tool() {
        // Measured against the live loop: asked for a "charting tool", `find`
        // returned nothing, because `"chart".contains("charting")` is false. Models
        // phrase requests as gerunds and plurals constantly, and one-directional
        // substring matching misses all of them.
        let held = vec![spec("chart", "files", 10), spec("sheet_read", "sheets", 10)];
        let (_, found) = reveal(&held, "charting tool");
        assert_eq!(found.first().map(|t| t.name.as_str()), Some("chart"));
        // The other direction still works, and so does the plural.
        assert!(!find(&held, "charts", 5).is_empty());
        assert!(!find(&held, "sheet", 5).is_empty());
    }

    #[test]
    pub(super) fn a_short_common_word_does_not_match_everything() {
        // The length floor on `akin` is what keeps "the" and "a" from scoring against
        // every tool in the catalogue and burying the real match.
        let held = vec![spec("chart", "files", 10), spec("board_add", "board", 10)];
        assert!(find(&held, "the a of", 5).is_empty());
    }

    #[test]
    pub(super) fn asking_for_something_that_is_not_there_says_what_is() {
        // A bare "no" costs a step and teaches the model nothing. Naming the
        // capabilities is a fraction of the tokens of their schemas.
        let held = vec![spec("sheet_read", "sheets", 10)];
        let (reply, revealed) = reveal(&held, "launch a rocket");
        assert!(revealed.is_empty());
        assert!(reply.contains("sheets"));
    }

    #[test]
    pub(super) fn the_find_tool_names_capabilities_rather_than_listing_schemas() {
        // If it cost what the schemas cost, withholding them would buy nothing.
        let held = vec![
            spec("sheet_read", "sheets", 3_000),
            spec("board_add", "board", 3_000),
        ];
        let t = find_tools_spec(&held);
        assert!(t.description.contains("sheets") && t.description.contains("board"));
        assert!(schema_tokens(&t) < 120, "cost {} tokens", schema_tokens(&t));
    }
