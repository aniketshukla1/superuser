use super::*;
    use serde_json::json;

    #[test]
    fn defaults_are_private_first() {
        let s = Settings::default();
        assert!(s.local_only);
        assert!(!s.cloud_enabled);
        assert_eq!(s.sync_interval_min, 15);
    }

    #[test]
    fn standing_instructions_arrive_with_the_rule_for_when_they_conflict() {
        let mut s = Settings::default();
        assert_eq!(s.standing_block(), "", "none written, nothing sent");
        s.apply_patch(&json!({ "standing_instructions": "  Use British spelling.  " }));
        let b = s.standing_block();
        assert!(b.contains("Use British spelling."), "{b}");
        assert!(b.contains("the role decides what you do and these decide how you do it"), "{b}");
        s.apply_patch(&json!({ "standing_instructions": "x".repeat(STANDING_INSTRUCTIONS_MAX + 500) }));
        assert_eq!(s.standing_instructions.chars().count(), STANDING_INSTRUCTIONS_MAX);
        s.apply_patch(&json!({ "standing_instructions": "" }));
        assert_eq!(s.standing_block(), "");
    }

    #[test]
    fn a_stall_limit_is_off_or_sane_never_a_second() {
        let mut s = Settings::default();
        s.apply_patch(&json!({ "stall_seconds": 1, "stall_seconds_local": 0 }));
        assert_eq!((s.stall_seconds, s.stall_seconds_local), (10, 0));
        s.apply_patch(&json!({ "stall_seconds": 90_000 }));
        assert_eq!(s.stall_seconds, 3600);
    }

    #[test]
    fn a_settings_file_from_before_the_trail_and_the_watchdog_still_reads() {
        let old = r#"{"local_only":true,"ollama_url":"http://localhost:11434","ollama_model":"llama3.2","embed_model":"nomic-embed-text","cloud_enabled":false,"cloud_model":"claude-opus-5"}"#;
        let s: Settings = serde_json::from_str(old).unwrap();
        assert_eq!(s.stall_seconds, crate::core::stall::DEFAULT_CLOUD_SECS);
        assert_eq!(s.stall_seconds_local, crate::core::stall::DEFAULT_LOCAL_SECS);
        assert_eq!(s.audit_retention_days, 0, "keep everything unless someone chose otherwise");
        assert!(s.standing_instructions.is_empty());
    }

    #[test]
    /// The door this was walked through: a chat model in the embedding slot is
    /// accepted by Ollama with a 501 and by the app with silence, and semantic
    /// recall never runs again.
    fn a_chat_model_cannot_be_stored_as_the_embedding_model() {
        let mut s = Settings::default();
        assert_eq!(s.embed_model, "nomic-embed-text", "the default must be able to embed");

        s.apply_patch(&json!({ "embed_model": "llama3.2:latest" }));
        assert_eq!(s.embed_model, "nomic-embed-text", "a chat model must be refused, not stored");

        // A real embedding model still goes through.
        s.apply_patch(&json!({ "embed_model": "mxbai-embed-large" }));
        assert_eq!(s.embed_model, "mxbai-embed-large");
    }

    #[test]
    fn patch_only_touches_present_keys() {
        let mut s = Settings::default();
        s.apply_patch(&json!({ "ollama_model": "qwen2.5", "sync_interval_min": 30 }));
        assert_eq!(s.ollama_model, "qwen2.5");
        assert_eq!(s.sync_interval_min, 30);
        assert!(s.local_only, "untouched field stays default");

        s.apply_patch(&json!({ "local_only": false }));
        assert!(!s.local_only);
        assert_eq!(s.ollama_model, "qwen2.5", "previous patch preserved");
    }

    #[test]
    fn context_and_token_windows_default_and_clamp() {
        let s = Settings::default();
        assert_eq!(s.num_ctx, 16384);
        assert_eq!(s.max_tokens, 4096);
        let mut s = Settings::default();
        s.apply_patch(&json!({ "num_ctx": 32768, "max_tokens": 8192 }));
        assert_eq!(s.num_ctx, 32768);
        assert_eq!(s.max_tokens, 8192);
        s.apply_patch(&json!({ "num_ctx": 999999999, "max_tokens": 0 }));
        assert_eq!(s.num_ctx, 131072, "clamped to max");
        assert_eq!(s.max_tokens, 256, "clamped to min");
    }

    #[test]
    fn agent_context_budget_defaults_and_clamps() {
        let s = Settings::default();
        assert_eq!(s.agent_max_context_tokens, 48000);
        assert_eq!(s.agent_max_context_turns, 20);
        assert_eq!(s.agent_max_steps, 24);

        let mut s = Settings::default();
        s.apply_patch(&json!({ "agent_max_context_tokens": 120000, "agent_max_context_turns": 40, "agent_max_steps": 40 }));
        assert_eq!(s.agent_max_context_tokens, 120000);
        assert_eq!(s.agent_max_context_turns, 40);
        assert_eq!(s.agent_max_steps, 40);

        // A zero budget would send an empty conversation and a zero step limit would
        // answer nothing — both clamp to their floor rather than bricking the turn.
        s.apply_patch(&json!({ "agent_max_context_tokens": 0, "agent_max_context_turns": 0, "agent_max_steps": 0 }));
        assert_eq!(s.agent_max_context_tokens, 4000);
        assert_eq!(s.agent_max_context_turns, 2);
        assert_eq!(s.agent_max_steps, 1);
    }

    #[test]
    fn settings_written_before_the_budget_existed_still_load() {
        // A settings.json from an older build has none of the agent_* keys.
        let old = r#"{"local_only":true,"ollama_url":"http://localhost:11434","ollama_model":"llama3.2","embed_model":"nomic-embed-text","cloud_enabled":false,"cloud_model":"claude-opus-4-8"}"#;
        let s: Settings = serde_json::from_str(old).expect("old settings still deserialize");
        assert_eq!(s.agent_max_context_tokens, 48000);
        assert_eq!(s.agent_max_steps, 24);
    }

    #[test]
    fn patch_ignores_wrong_types_and_unknown_keys() {
        let mut s = Settings::default();
        s.apply_patch(&json!({ "local_only": "yes", "bogus": 1, "sync_interval_min": "x" }));
        assert!(s.local_only); // string ignored, stays default
        assert_eq!(s.sync_interval_min, 15);
    }
