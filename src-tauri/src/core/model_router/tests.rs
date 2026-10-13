use super::*;

    const WANTS_CLOUD: Workload = Workload::Foreground(true);

    #[test]
    fn local_only_never_routes_to_cloud() {
        let mut s = Settings::default();
        s.cloud_enabled = true;
        s.local_only = true;
        assert!(matches!(route(&s, WANTS_CLOUD).0, Provider::Ollama));
    }

    #[test]
    fn cloud_requires_enable_and_not_local_only() {
        let mut s = Settings::default();
        s.local_only = false;
        s.cloud_enabled = false;
        assert!(matches!(route(&s, WANTS_CLOUD).0, Provider::Ollama));
    }

    #[test]
    fn route_prefers_local_when_no_cloud_key() {
        let mut s = Settings::default();
        s.local_only = false;
        s.cloud_enabled = true; // enabled, but no key in keychain
        assert!(matches!(route(&s, WANTS_CLOUD).0, Provider::Ollama));
    }

    #[test]
    fn background_work_takes_the_small_model_when_there_is_one() {
        let mut s = Settings::default();
        s.ollama_model = "qwen2.5:14b".into();
        s.small_model = "qwen2.5:1.5b".into();

        // The user's turn keeps the model they chose…
        assert_eq!(route(&s, Workload::Foreground(false)).1, "qwen2.5:14b");
        // …while the loops that run on their own stop competing for it.
        assert_eq!(route(&s, Workload::Background).1, "qwen2.5:1.5b");
    }

    #[test]
    fn without_a_small_model_nothing_changes() {
        // The whole point of the empty default: an install that never touches the
        // setting behaves exactly as it did before this existed.
        let mut s = Settings::default();
        s.ollama_model = "qwen2.5:14b".into();
        assert_eq!(s.small_model, "");
        assert_eq!(route(&s, Workload::Background).1, "qwen2.5:14b");
        // Whitespace is not a model name either.
        s.small_model = "   ".into();
        assert_eq!(route(&s, Workload::Background).1, "qwen2.5:14b");
    }

    #[test]
    fn background_work_follows_the_model_the_user_is_actually_talking_to() {
        // The eviction bug, pinned. `ollama_model` is not "the chat model" whenever a
        // local `cloud_model` is set and ready — the turn loop resolves through
        // `effective_model` and gets the other one. Routing background work to
        // `ollama_model` then loaded a SECOND multi-gigabyte model, evicting the one
        // the user was mid-conversation with and costing 18–57s on their next message.
        //
        // The owner's real configuration is `custom:qwen2.5:7b-instruct` against
        // `gemma4:12b`; the prefix is stripped here because Ollama's API knows the
        // tag and not the provider id.
        assert_eq!(
            pick_local_chat_model("custom:qwen2.5:7b-instruct", true, "gemma4:12b"),
            "qwen2.5:7b-instruct",
            "background work must reuse the resident model, not load a second one"
        );
        // A bare Ollama tag is local too, and its colon is a TAG, not a provider
        // prefix — stripping it would ask for a model called `12b`.
        assert_eq!(pick_local_chat_model("gemma4:12b", true, "llama3.2:latest"), "gemma4:12b");
    }

    #[test]
    fn a_cloud_chat_model_leaves_background_work_on_the_local_one() {
        // The other direction, and the reason this is not simply `effective_model()`:
        // background work never goes to the cloud, so a user whose chat runs on
        // Claude must still have their loops run on the local model.
        assert_eq!(pick_local_chat_model("claude-opus-5", false, "gemma4:12b"), "gemma4:12b");
        assert_eq!(pick_local_chat_model("openai:gpt-5.5", false, "gemma4:12b"), "gemma4:12b");

        // …and end to end, through the real router.
        let mut s = Settings::default();
        s.local_only = false;
        s.cloud_enabled = true;
        s.cloud_model = "claude-opus-5".into();
        s.ollama_model = "gemma4:12b".into();
        let (provider, model) = route(&s, Workload::Background);
        assert!(matches!(provider, Provider::Ollama));
        assert_eq!(model, "gemma4:12b");
    }

    #[test]
    fn the_passes_that_rewrite_memory_keep_the_full_model() {
        // Deep Dream and the evolution review discard answers that miss their exact
        // output format, so routing them to a 1.5b would make them silently no-ops.
        let mut s = Settings::default();
        s.ollama_model = "qwen2.5:14b".into();
        s.small_model = "qwen2.5:1.5b".into();
        assert_eq!(route(&s, Workload::BackgroundCapable).1, "qwen2.5:14b");
    }

    #[test]
    fn background_never_reaches_the_cloud_however_it_is_configured() {
        // Nobody should be billed for a loop they aren't watching.
        let mut s = Settings::default();
        s.local_only = false;
        s.cloud_enabled = true;
        for w in [Workload::Background, Workload::BackgroundCapable] {
            assert!(matches!(route(&s, w).0, Provider::Ollama));
        }
    }

    // ---- end-to-end against a real Ollama (run: cargo test -- --ignored) ----

    #[tokio::test]
    #[ignore = "requires a running Ollama with a chat model"]
    async fn e2e_ollama_chat_responds() {
        let s = Settings::default(); // configured model may be missing — resolver falls back
        let msgs = vec![ChatMessage { role: "user".into(), content: "Reply with exactly the word: pong".into(), images: vec![], docs: vec![] }];
        let (content, provider) = ollama(&s, &msgs, &s.ollama_model).await.expect("ollama chat should succeed");
        assert!(!content.trim().is_empty(), "expected a non-empty reply");
        assert!(provider.starts_with("ollama:"), "provider was {provider}");
    }

    #[tokio::test]
    #[ignore = "requires a running Ollama"]
    async fn e2e_ollama_unreachable_url_errors_clearly() {
        let mut s = Settings::default();
        s.ollama_url = "http://127.0.0.1:1".into(); // nothing listening
        let err = ollama(&s, &[ChatMessage { role: "user".into(), content: "hi".into(), images: vec![], docs: vec![] }], &s.ollama_model).await.unwrap_err();
        assert!(err.to_lowercase().contains("ollama"), "error was: {err}");
    }
