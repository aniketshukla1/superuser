use super::*;

    #[test]
    pub(super) fn anthropic_capabilities_are_parsed_from_the_documented_shape() {
        // The response schema from the live API reference, read 2026-09-02. This is
        // the authoritative version of `core::effort`'s hand-maintained tables.
        let entry = json!({
            "id": "claude-opus-5",
            "type": "model",
            "display_name": "Claude Opus 5",
            "created_at": "2026-07-24T00:00:00Z",
            "max_input_tokens": 200000,
            "max_tokens": 64000,
            "capabilities": {
                "image_input": { "supported": true },
                "pdf_input": { "supported": true },
                "structured_outputs": { "supported": true },
                "thinking": { "supported": true, "types": {
                    "adaptive": { "supported": true },
                    "enabled": { "supported": true }
                }},
                "effort": { "supported": true, "low": {"supported": true},
                            "medium": {"supported": true}, "high": {"supported": true},
                            "max": {"supported": true}, "xhigh": {"supported": true} }
            }
        });
        let p = parse_anthropic_model(&entry).unwrap();
        assert_eq!(p.context_window, Some(200_000));
        assert_eq!(p.max_output, Some(64_000));
        assert_eq!(p.thinking, Some(ThinkingStyle::Adaptive));
        assert!(p.vision && p.pdf && p.strict_schemas);
        assert_eq!(p.caching, Some(CacheStyle::Explicit));
        assert_eq!(p.source, ProfileSource::Discovered);
    }

    #[test]
    pub(super) fn the_documented_placeholder_zero_is_read_as_unknown() {
        // The reference's own example ships `"max_input_tokens": 0`. Believed, that is
        // a model that can hold nothing, and every conversation gets trimmed away.
        let entry = json!({ "id": "claude-opus-5", "max_input_tokens": 0, "max_tokens": 0 });
        let p = parse_anthropic_model(&entry).unwrap();
        assert_eq!(p.context_window, None);
        assert_eq!(p.max_output, None);
    }

    #[test]
    pub(super) fn a_model_with_only_the_older_thinking_type_gets_a_budget() {
        let entry = json!({ "id": "claude-haiku-4-5", "capabilities": {
            "thinking": { "supported": true, "types": {
                "adaptive": { "supported": false }, "enabled": { "supported": true } } } } });
        assert_eq!(parse_anthropic_model(&entry).unwrap().thinking, Some(ThinkingStyle::Budget));
        // And a model that reports no thinking at all is not asked for it. Phase 1
        // found this live: a `think` field the model rejects loses the whole turn.
        let entry = json!({ "id": "x", "capabilities": { "thinking": { "supported": false } } });
        assert_eq!(parse_anthropic_model(&entry).unwrap().thinking, None);
    }

    #[test]
    pub(super) fn the_declared_table_agrees_with_the_effort_tables_it_reads_from() {
        // One source of truth: `declared` asks `core::effort` rather than keeping a
        // second list that can drift from it.
        let p = declared("claude-opus-5").unwrap();
        assert_eq!(p.thinking, Some(ThinkingStyle::Adaptive));
        assert_eq!(p.source, ProfileSource::Declared);
        assert_eq!(p.tier, Tier::Frontier);
        assert_eq!(p.caching, Some(CacheStyle::Explicit));
        // And it declines to speak for an id we do not ship, rather than guessing
        // from the provider it happens to sit under.
        assert!(declared("anthropic:claude-imaginary-9").is_none());
    }

    #[test]
    pub(super) fn declared_leaves_the_context_window_to_the_api() {
        // A hardcoded 200k is folklore with an expiry date. `/v1/models` has the real
        // number; until it answers, `None` means the configured budget applies —
        // which is exactly today's behaviour.
        assert_eq!(declared("claude-opus-5").unwrap().context_window, None);
    }

    #[test]
    pub(super) fn a_small_variant_is_not_given_the_frontier_loop_shape() {
        assert_eq!(cloud_tier("claude-haiku-4-5"), Tier::Mid);
        assert_eq!(cloud_tier("openai:gpt-5-mini"), Tier::Mid);
        assert_eq!(cloud_tier("gemini-2.5-flash"), Tier::Mid);
        assert_eq!(cloud_tier("claude-opus-5"), Tier::Frontier);
        assert_eq!(cloud_tier("openai:gpt-5.5"), Tier::Frontier);
        // An id nobody ships support for gets the middle, not the top.
        assert_eq!(cloud_tier("custom:something-unknown"), Tier::Mid);
    }

    #[test]
    pub(super) fn a_local_model_never_gets_the_frontier_loop_shape() {
        // Frontier means "generous": more tools, more steps, a longer prompt. A 70B on
        // a laptop needs the opposite of that, however capable it is.
        assert_eq!(local_tier(4_683_087_332), Tier::Small);
        assert_eq!(local_tier(7_556_508_396), Tier::Mid);
        assert_eq!(local_tier(40_000_000_000), Tier::Mid);
    }

    #[test]
    pub(super) fn the_probe_tells_sequential_from_parallel_and_from_neither() {
        let two = json!({ "choices": [{ "message": { "tool_calls": [
            { "function": { "name": "ping" } }, { "function": { "name": "pong" } } ] } }],
            "usage": { "prompt_tokens": 41, "completion_tokens": 8 } });
        assert_eq!(read_probe(&two), (ToolSupport::Parallel, true));

        let one = json!({ "choices": [{ "message": { "tool_calls": [
            { "function": { "name": "ping" } } ] } }] });
        assert_eq!(read_probe(&one), (ToolSupport::Sequential, false));

        let none = json!({ "choices": [{ "message": { "content": "pong" } }] });
        assert_eq!(read_probe(&none), (ToolSupport::None, false));
    }

    #[test]
    pub(super) fn an_empty_usage_object_is_not_a_server_that_reports_usage() {
        // A blank and a zero are different facts — the same rule `TokenUsage` and
        // `core::pricing` already follow. A server echoing `usage: {}` reports
        // nothing, and recording it as reporting puts a false zero in the totals.
        let v = json!({ "choices": [{ "message": { "tool_calls": [] } }], "usage": {} });
        assert_eq!(read_probe(&v).1, false);
    }

    #[test]
    pub(super) fn an_unprobed_endpoint_is_never_sent_a_reasoning_parameter() {
        // The live failure this closes: `custom:qwen2.5:7b-instruct` answers 400 to
        // `reasoning_effort`, and a 400 discards the whole turn — every tool call it
        // had already run. The `custom` provider ships no model list, so nothing
        // could ever have declared this model's capabilities; the only safe default
        // is to send nothing until a handshake says otherwise.
        assert_eq!(assumed("custom:qwen2.5:7b-instruct").thinking, None);
        assert!(declared("custom:qwen2.5:7b-instruct").is_none());
    }

    #[test]
    pub(super) fn the_conservative_default_can_still_run_a_turn() {
        // Conservative is about claims, not capacity. Asserting thinking or caching
        // wrongly fails a turn; leaving tools off would fail every turn.
        let p = assumed("custom:whatever");
        assert_eq!(p.source, ProfileSource::Assumed);
        assert!(p.can_run_a_turn());
        assert!(!p.can_think());
        assert_eq!(p.caching, None);
        assert!(!p.vision);
    }

    #[test]
    pub(super) fn a_window_reads_as_a_size_a_person_recognises() {
        assert_eq!(human_tokens(262_144), "262k");
        assert_eq!(human_tokens(32_768), "32k");
        assert_eq!(human_tokens(1_048_576), "1M");
        assert_eq!(human_tokens(512), "512");
        // Rounded down. "263k" for a model that holds 262,144 is the same overstatement
        // as a setting that asks for more than the window, one screen earlier.
        assert_eq!(human_tokens(131_072), "131k");
    }

    #[test]
    pub(super) fn a_guess_is_never_cached_in_place_of_an_answer() {
        // The cache holds evidence only. Caching a fallback would keep a guess alive
        // for a quarter of an hour after the server that could answer came back.
        assert!(ProfileSource::Discovered.is_evidence());
        assert!(ProfileSource::Probed.is_evidence());
        assert!(!ProfileSource::Declared.is_evidence());
        assert!(!ProfileSource::Assumed.is_evidence());
    }

    #[test]
    pub(super) fn cache_identity_includes_the_address_not_just_the_model() {
        // The same id behind a different gateway is a different model. Keyed on the
        // id alone, one user's vLLM profile would describe OpenAI's server.
        assert_ne!(
            cache_key("https://api.openai.com/v1", "gpt-5"),
            cache_key("http://127.0.0.1:8000/v1", "gpt-5")
        );
    }
