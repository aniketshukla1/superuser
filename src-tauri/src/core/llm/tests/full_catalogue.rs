use super::*;

    #[test]
    pub(super) fn a_request_carries_only_the_fields_it_was_given() {
        // `tools: []` must not become `"tools": []` on the wire: an empty array is a
        // claim ("this turn has no tools"), and the callers that never had tools were
        // not making it.
        let req = LlmRequest::new("claude-opus-5", 4096).messages(vec![json!({"role":"user","content":"hi"})]);
        let body = req.body();
        assert_eq!(body["model"], "claude-opus-5");
        assert_eq!(body["max_tokens"], 4096);
        assert!(body.get("tools").is_none());
        assert!(body.get("system").is_none());
        assert!(body.get("thinking").is_none(), "no effort asked for, no thinking field");
    }

    #[test]
    pub(super) fn effort_becomes_whatever_this_model_accepts() {
        // The §2.1 fix, reachable from the one place every caller now goes through.
        let modern = LlmRequest::new("claude-opus-5", 32000).effort("xhigh").body();
        assert_eq!(modern["thinking"]["type"], "adaptive");
        assert!(modern["thinking"].get("budget_tokens").is_none(), "would 400");
        assert_eq!(modern["output_config"]["effort"], "xhigh");

        // And the reverse: 4.5 takes the old mode and would 400 on adaptive.
        let older = LlmRequest::new("claude-haiku-4-5-20251001", 32000).effort("high").body();
        assert_eq!(older["thinking"]["type"], "enabled");
        assert!(older["thinking"]["budget_tokens"].as_u64().unwrap() >= 1024);
        assert!(older.get("output_config").is_none());
    }

    #[test]
    pub(super) fn usage_keeps_the_cache_split_and_names_its_model() {
        let v = json!({"usage": {
            "input_tokens": 12, "output_tokens": 3,
            "cache_read_input_tokens": 900, "cache_creation_input_tokens": 100
        }});
        let u = usage_from(&v, "claude-opus-5").expect("usage");
        assert_eq!((u.input, u.output, u.cache_read, u.cache_write), (12, 3, 900, 100));
        assert_eq!(u.model.as_deref(), Some("claude-opus-5"));
        // Silence is not a free turn.
        assert!(usage_from(&json!({}), "claude-opus-5").is_none());
        assert!(usage_from(&json!({"usage": {}}), "claude-opus-5").is_none());
    }

    #[test]
    pub(super) fn a_reply_separates_what_was_said_from_what_was_asked_for() {
        let reply = LlmReply {
            content: vec![
                json!({"type":"text","text":"Looking that up."}),
                json!({"type":"tool_use","id":"t1","name":"web_search","input":{}}),
                json!({"type":"text","text":"Done."}),
            ],
            stop_reason: "tool_use".into(),
            usage: None,
        };
        assert_eq!(reply.text(), "Looking that up.\nDone.");
        assert_eq!(reply.tool_uses().len(), 1);
        assert_eq!(reply.tool_uses()[0]["name"], "web_search");
    }

    /// The whole shipped catalogue, as `tools_claude` would send it.
    pub(super) fn full_catalogue() -> Vec<Value> {
        use crate::core::agent::{expand, AgentContext};
        let ctx = AgentContext {
            workspace: Some(std::path::PathBuf::from("/tmp/ws")),
            roots: vec![std::path::PathBuf::from("/tmp/ws")],
            has_executor: true,
            has_todo: true,
        };
        let caps: Vec<String> = [
            "files", "search", "shell", "git", "todo", "web", "toolchain", "board",
            "subagent", "knowledge", "recall_memory", "connectors", "messaging",
            "schedule", "computer", "plan", "ask",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        expand(&caps, &ctx)
            .iter()
            .map(|t| json!({"name": t.name, "description": t.description, "input_schema": t.schema}))
            .collect()
    }

    pub(super) fn tokens(v: &[Value]) -> usize {
        serde_json::to_string(v).unwrap().len() / 4
    }

    #[test]
    pub(super) fn the_resident_schema_fits_the_budget_phase_6_set() {
        // HARNESS-PLAN §2.13's exit criterion: resident tool schema under 3k tokens.
        // Measured, because the plan's own starting figure (10.2k) did not match what
        // this catalogue actually weighs (~6.5k) and a target checked against the
        // wrong baseline is not a target.
        let all = full_catalogue();
        let before = tokens(&all);
        let req = LlmRequest::new("claude-opus-5", 4096).tools(all.clone());
        let wire = req.deferred_tools();
        let resident: Vec<Value> = wire
            .iter()
            .filter(|t| t.get("defer_loading") != Some(&json!(true)))
            .cloned()
            .collect();
        let after = tokens(&resident);
        eprintln!("TOOL SCHEMA: {before} tok before, {after} tok resident ({} of {} tools)", resident.len(), all.len());
        assert!(after < 3000, "resident schema is {after} tokens, over the 3k budget");
        assert!(after < before / 2, "{after} is not a meaningful cut from {before}");
    }

    #[test]
    pub(super) fn the_search_tool_is_present_and_never_deferred() {
        // "At least one tool must have defer_loading=false" — and the API rejects a
        // request where everything is deferred, which is what deferring the search
        // tool itself would produce.
        let wire = LlmRequest::new("claude-opus-5", 4096).tools(full_catalogue()).deferred_tools();
        let search = &wire[0];
        assert_eq!(search["type"], "tool_search_tool_regex_20251119");
        assert!(search.get("defer_loading").is_none());
        assert!(wire.iter().any(|t| t.get("defer_loading") != Some(&json!(true))));
    }

    #[test]
    pub(super) fn a_model_without_tool_search_gets_the_plain_list() {
        // Sonnet 5 is absent from the support table — between Sonnet 4.5 and Sonnet
        // 4.6, which is exactly the gap version arithmetic would paper over. Sending
        // it a search tool would 400 every turn.
        assert!(!supports_tool_search("claude-sonnet-5"));
        let all = full_catalogue();
        let wire = LlmRequest::new("claude-sonnet-5", 4096).tools(all.clone()).deferred_tools();
        assert_eq!(wire.len(), all.len(), "no search tool added");
        assert!(wire.iter().all(|t| t.get("defer_loading").is_none()));

        assert!(!supports_tool_search("gemma4:12b"));
        for m in ["claude-opus-5", "anthropic:claude-opus-4-8", "claude-haiku-4-5-20251001"] {
            assert!(supports_tool_search(m), "{m}");
        }
    }

    #[test]
    pub(super) fn a_short_tool_list_is_left_alone() {
        // Under about ten tools, searching for one costs more than carrying them all.
        let few: Vec<Value> = (0..5).map(|i| json!({"name": format!("t{i}"), "input_schema": {}})).collect();
        let wire = LlmRequest::new("claude-opus-5", 4096).tools(few.clone()).deferred_tools();
        assert_eq!(wire.len(), few.len());
    }

    #[test]
    pub(super) fn the_breakpoint_sits_after_the_stable_half_and_nowhere_else() {
        // The whole of §2.8 in one assertion. A breakpoint on content that changes
        // per request pays the 1.25× write premium every time and reads nothing back
        // — which is what marking up the old ordering would have done, because the
        // SuperContext brief and a minute-resolution clock sat in FRONT of the
        // persona, the gates, the skills and AGENTS.md.
        let p = SystemPrompt {
            stable: "persona, gates, skills, AGENTS.md".into(),
            volatile: "\n\nopen documents, the brief, and the time".into(),
        };
        let blocks = p.blocks();
        let arr = blocks.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["cache_control"]["type"], "ephemeral");
        assert!(arr[0]["text"].as_str().unwrap().contains("persona"));
        // And explicitly NOT on the half that changes.
        assert!(arr[1].get("cache_control").is_none(), "the volatile half must not be a breakpoint");
        assert!(arr[1]["text"].as_str().unwrap().contains("brief"));
    }

    #[test]
    pub(super) fn an_empty_volatile_half_does_not_become_an_empty_block() {
        // "Empty text blocks cannot be cached" — and an empty trailing block is a
        // request the API can reject for nothing gained.
        let p = SystemPrompt { stable: "just the stable part".into(), volatile: "   \n".into() };
        let arr = p.blocks();
        assert_eq!(arr.as_array().unwrap().len(), 1);
    }

    #[test]
    pub(super) fn the_two_halves_still_read_as_one_prompt() {
        // Every other provider gets the same text it always did. The split is about
        // where a marker goes, not about changing what the model is told.
        let p = SystemPrompt { stable: "A".into(), volatile: "B".into() };
        assert_eq!(p.joined(), "AB");
    }
