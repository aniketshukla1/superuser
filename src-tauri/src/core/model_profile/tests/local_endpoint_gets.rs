use super::*;

    pub(super) fn ollama_tags() -> Vec<ModelInfo> {
        crate::core::ollama::parse_model_infos(&json!({ "models": [
            { "name": "gemma4:12b", "size": 7_556_508_396u64,
              "details": { "context_length": 262144 },
              "capabilities": ["completion", "tools", "thinking", "vision"] },
            { "name": "qwen2.5:7b-instruct", "size": 4_683_087_332u64,
              "details": { "context_length": 32768 },
              "capabilities": ["completion", "tools"] },
            { "name": "nomic-embed-text:latest", "size": 274_302_450u64,
              "details": { "context_length": 2048 },
              "capabilities": ["embedding"] },
            // An Ollama old enough not to report the window at all.
            { "name": "ancient:1b", "size": 1_000_000_000u64 },
        ]}))
    }

    #[test]
    pub(super) fn a_models_real_window_comes_from_the_tag_not_from_a_setting() {
        // The measured numbers on this machine, and the reason H4 is a finding: the
        // shipped `num_ctx` is 16384 for all four of these.
        let infos = ollama_tags();
        let g = from_ollama("gemma4:12b", &infos).unwrap();
        assert_eq!(g.context_window, Some(262_144));
        let q = from_ollama("qwen2.5:7b-instruct", &infos).unwrap();
        assert_eq!(q.context_window, Some(32_768));
    }

    #[test]
    pub(super) fn an_unreported_window_is_none_and_never_zero() {
        // A model on an Ollama that predates `details.context_length`. Unknown must
        // stay unknown: read as a number, it would be the smallest window in the list
        // and would trim every conversation to nothing.
        let p = from_ollama("ancient:1b", &ollama_tags()).unwrap();
        assert_eq!(p.context_window, None);
        let s = Settings::default();
        assert_eq!(p.ollama_num_ctx(&s), s.num_ctx, "falls back to the setting");
    }

    #[test]
    pub(super) fn vision_is_read_from_the_capability_not_guessed_from_the_name() {
        // The bug this fixes, exactly: `gemma4:12b` HAS vision and the name heuristic
        // in `ollama::is_vision_model` says it does not — its keyword list has
        // "gemma3". A user on gemma4 who attaches an image is silently switched to
        // another model, and computer use refuses the model outright.
        let infos = ollama_tags();
        assert!(!crate::core::ollama::is_vision_model("gemma4:12b"));
        assert!(from_ollama("gemma4:12b", &infos).unwrap().vision);
        assert!(!from_ollama("qwen2.5:7b-instruct", &infos).unwrap().vision);
    }

    #[test]
    pub(super) fn an_ollama_too_old_to_report_capabilities_does_not_refuse_every_turn() {
        // The trap in reading a capability list literally: absent and empty are the
        // same `false`, so on an Ollama that predates `capabilities` in `/api/tags`
        // every model reads as chat-only — and the caller acts on that by refusing
        // the turn before it starts. Unknown has to mean unknown.
        let p = from_ollama("ancient:1b", &ollama_tags()).unwrap();
        assert_eq!(p.tools, ToolSupport::Sequential);
        assert!(p.can_run_a_turn());
        // …and it says so, rather than passing a guess off as a discovery.
        assert_eq!(p.source, ProfileSource::Assumed);
        // The window it *did* report is still believed — that comes from `details`,
        // not from the capability list.
        assert_eq!(
            from_ollama("ancient:1b", &ollama_tags()).unwrap().context_window,
            None
        );
    }

    #[test]
    pub(super) fn a_named_vision_model_on_an_old_ollama_is_still_recognised() {
        // Where there is no capability list, the name is the only evidence there is,
        // and answering `false` for `llava` would be worse than the heuristic.
        let infos = crate::core::ollama::parse_model_infos(&json!({ "models": [
            { "name": "llava:7b", "size": 4_000_000_000u64 },
        ]}));
        assert!(from_ollama("llava:7b", &infos).unwrap().vision);
    }

    #[test]
    pub(super) fn a_chat_only_model_is_marked_as_unable_to_run_a_turn() {
        let p = from_ollama("nomic-embed-text:latest", &ollama_tags()).unwrap();
        assert_eq!(p.tools, ToolSupport::None);
        assert!(!p.can_run_a_turn());
    }

    #[test]
    pub(super) fn thinking_is_a_capability_read_on_every_kind() {
        let infos = ollama_tags();
        assert!(from_ollama("gemma4:12b", &infos).unwrap().can_think());
        assert!(!from_ollama("qwen2.5:7b-instruct", &infos).unwrap().can_think());
    }

    #[test]
    pub(super) fn num_ctx_is_clamped_to_the_window_and_never_raised_to_it() {
        // Both directions matter and they are different bugs. Down: a 32k model asked
        // to hold 128k. Up: a 262k model on a 16 GB laptop must NOT be handed 262144,
        // because Ollama allocates the KV cache for whatever it is told.
        let infos = ollama_tags();
        let mut s = Settings::default();
        s.num_ctx = 131_072;
        assert_eq!(
            from_ollama("qwen2.5:7b-instruct", &infos).unwrap().ollama_num_ctx(&s),
            32_768,
            "clamped down to what the model can hold"
        );
        s.num_ctx = 16_384;
        assert_eq!(
            from_ollama("gemma4:12b", &infos).unwrap().ollama_num_ctx(&s),
            16_384,
            "the user's memory setting is not overridden by a big window"
        );
    }

    #[test]
    pub(super) fn the_input_budget_never_exceeds_what_the_backend_will_hold() {
        // H4: 48000 configured against a 16384 window is a budget that does not exist,
        // and Ollama answers an overflow by dropping the front — the system prompt.
        let infos = ollama_tags();
        let mut s = Settings::default();
        s.agent_max_context_tokens = 48_000;
        s.num_ctx = 16_384;
        s.max_tokens = 4_096;
        let b = from_ollama("gemma4:12b", &infos).unwrap().input_budget(&s);
        assert_eq!(b, 16_384 - 4_096);
        // Raise the allocation and the model's own window becomes the binding limit.
        s.num_ctx = 131_072;
        let b = from_ollama("qwen2.5:7b-instruct", &infos).unwrap().input_budget(&s);
        assert_eq!(b, 32_768 - 4_096);
    }

    #[test]
    pub(super) fn a_local_endpoint_gets_the_allocation_ceiling_whatever_its_wire_format_is() {
        // The bug: the ceiling was gated on `kind == ProviderKind::Ollama`, so a
        // local server speaking the OpenAI wire format — which is how a `custom:`
        // endpoint reaches Ollama, and how the owner's own machine is set up —
        // skipped it entirely. With no declared window either, `agent_max_context_tokens`
        // won outright, and the app assembled 48,000 tokens for a runner that could
        // hold far less. Ollama does not refuse an overflow, it drops the front of
        // the prompt: the model silently loses its own system instructions, having
        // first been made to prefill tens of thousands of tokens at ~150 tok/s.
        //
        // A bare tag keeps this deterministic: `runs_locally` answers from the id,
        // so the test does not consult the machine's provider-override store.
        let mut p = assumed("qwen2.5:7b-instruct");
        p.kind = ProviderKind::OpenAiCompatible;
        assert_eq!(p.context_window, None, "the case that made the bug reachable");

        let mut s = Settings::default();
        s.num_ctx = 16_384;
        s.max_tokens = 4_096;
        s.agent_max_context_tokens = 48_000;

        assert_eq!(
            p.input_budget(&s),
            12_288,
            "num_ctx minus the reply reservation, not the configured 48k"
        );
    }

    #[test]
    pub(super) fn a_cloud_profile_has_no_local_allocation_ceiling() {
        // `num_ctx` is an Ollama memory setting. Letting it bound a cloud model would
        // cap Claude at the laptop's KV budget for no reason.
        let mut p = assumed("claude-opus-5");
        p.kind = ProviderKind::Anthropic;
        p.context_window = Some(200_000);
        let mut s = Settings::default();
        s.num_ctx = 8_192;
        s.agent_max_context_tokens = 48_000;
        assert_eq!(p.input_budget(&s), 48_000);
    }
