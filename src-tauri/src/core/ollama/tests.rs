use super::*;
    fn m(v: &[&str]) -> Vec<String> { v.iter().map(|s| s.to_string()).collect() }

    #[test]
    fn thinking_is_only_asked_of_models_that_advertise_it() {
        // `think: true` sent to a model without the capability makes Ollama reject the
        // request outright — the turn fails instead of merely lacking a nicety. So the
        // gate has to be a real capability check, and an unknown model must answer no.
        let tags = serde_json::json!({ "models": [
            { "name": "gemma4:12b",         "size": 7_600_000_000u64, "capabilities": ["completion", "tools", "thinking"] },
            { "name": "qwen2.5:7b-instruct","size": 4_700_000_000u64, "capabilities": ["completion", "tools"] },
            { "name": "ancient:1b",         "size": 1_000_000_000u64 },
        ]});
        let infos = parse_model_infos(&tags);

        assert!(supports_thinking(&infos, "gemma4:12b"));
        assert!(!supports_thinking(&infos, "qwen2.5:7b-instruct"));
        // An older Ollama that reports no capabilities at all.
        assert!(!supports_thinking(&infos, "ancient:1b"));
        // A model we've never heard of must not be guessed at.
        assert!(!supports_thinking(&infos, "something:else"));

        // The tools capability keeps working alongside it.
        assert!(infos.iter().find(|m| m.name == "gemma4:12b").unwrap().tools);
        assert!(!infos.iter().find(|m| m.name == "ancient:1b").unwrap().tools);
    }

    #[test]
    fn base_name_strips_tag() {
        assert_eq!(base_name("llama3.2:latest"), "llama3.2");
        assert_eq!(base_name("qwen2.5:7b-instruct"), "qwen2.5");
        assert_eq!(base_name("mistral"), "mistral");
    }

    #[test]
    fn availability_matches_base_and_exact() {
        let models = m(&["llama3.2:latest", "qwen2.5:7b-instruct"]);
        assert!(model_available(&models, "llama3.2"));
        assert!(model_available(&models, "llama3.2:latest"));
        assert!(model_available(&models, "qwen2.5"));
        assert!(model_available(&models, "qwen2.5:7b-instruct"));
        assert!(!model_available(&models, "llama3.1"));
        assert!(!model_available(&models, ""));
    }

    #[test]
    fn resolve_prefers_wanted_then_non_embed() {
        let models = m(&["nomic-embed-text:latest", "llama3.2:latest"]);
        assert_eq!(resolve_chat_model(&models, "llama3.2").as_deref(), Some("llama3.2:latest"));
        // wanted missing -> first non-embed
        assert_eq!(resolve_chat_model(&models, "mistral").as_deref(), Some("llama3.2:latest"));
        // only an embed model installed -> None usable for chat
        assert_eq!(resolve_chat_model(&m(&["nomic-embed-text:latest"]), "mistral"), None);
        // nothing installed
        assert_eq!(resolve_chat_model(&[], "x"), None);
    }

    fn infos(v: &[(&str, u64, bool)]) -> Vec<ModelInfo> {
        v.iter().map(|(n, s, t)| ModelInfo { name: n.to_string(), size: *s, tools: *t, thinking: false, vision: false, context_length: None, reported_capabilities: true }).collect()
    }

    /// The machine this was found on, in listing order. `llama3.1` was configured but never
    /// installed, and the old "first non-embedding" rule handed every turn to a 19.7 GB
    /// no-tools model on a 16 GB laptop.
    #[test]
    fn fallback_picks_a_model_that_can_actually_run_the_turn() {
        let installed = infos(&[
            ("hf.co/unsloth/Muse-Glimmer-30B-GGUF:UD-Q4_K_XL", 19_700_000_000, false),
            ("gemma4:12b", 7_600_000_000, true),
            ("llama3.2-vision:latest", 7_800_000_000, false),
            ("nomic-embed-text:latest", 300_000_000, false),
            ("qwen2.5:7b-instruct", 4_700_000_000, true),
            ("llama3.2:latest", 2_000_000_000, true),
        ]);

        // Not the 19.7 GB one that merely happened to be listed first.
        assert_eq!(
            resolve_chat_model_info(&installed, "llama3.1").as_deref(),
            Some("llama3.2:latest"),
            "fallback must be the smallest tool-capable model, not the first listed",
        );

        // An explicit choice is honored exactly — ranking applies only to fallbacks.
        assert_eq!(
            resolve_chat_model_info(&installed, "qwen2.5:7b-instruct").as_deref(),
            Some("qwen2.5:7b-instruct"),
        );
        assert_eq!(resolve_chat_model_info(&installed, "gemma4").as_deref(), Some("gemma4:12b"));
    }

    #[test]
    fn fallback_degrades_sanely_without_tool_capable_models() {
        // No tool support anywhere (or an older Ollama that reports no capabilities):
        // still avoid embeddings, still prefer the one that will actually load.
        let no_tools = infos(&[("big:70b", 40_000_000_000, false), ("small:3b", 2_000_000_000, false)]);
        assert_eq!(resolve_chat_model_info(&no_tools, "missing").as_deref(), Some("small:3b"));

        // Embedding-only is not a chat model.
        assert_eq!(resolve_chat_model_info(&infos(&[("nomic-embed-text:latest", 3, false)]), "x"), None);
        assert_eq!(resolve_chat_model_info(&[], "x"), None);
    }

    #[test]
    fn tags_parsing_keeps_size_and_tool_capability() {
        let tags = serde_json::json!({ "models": [
            { "name": "gemma4:12b", "size": 7_556_508_396u64, "capabilities": ["completion", "tools", "vision"] },
            { "name": "llama3.2-vision:latest", "size": 7_816_589_186u64, "capabilities": ["completion", "vision"] },
            { "name": "old-server-model", "size": 1000 },
        ]});
        let got = parse_model_infos(&tags);
        assert_eq!(got.len(), 3);
        assert!(got[0].tools);
        assert_eq!(got[0].size, 7_556_508_396);
        assert!(!got[1].tools);
        assert!(!got[2].tools, "a server that reports no capabilities must not claim tool support");
    }

    #[test]
    fn embed_detection() {
        assert!(is_embed_model("nomic-embed-text:latest"));
        assert!(is_embed_model("bge-m3"));
        assert!(!is_embed_model("llama3.2:latest"));
        assert!(!is_embed_model("qwen2.5:7b-instruct"));
    }

    #[test]
    fn parse_models_from_tags() {
        let tags = serde_json::json!({ "models": [{"name": "llama3.2:latest"}, {"name": "qwen2.5:7b-instruct"}] });
        assert_eq!(parse_models(&tags), m(&["llama3.2:latest", "qwen2.5:7b-instruct"]));
        assert!(parse_models(&serde_json::json!({})).is_empty());
    }

    /// The whole point of `stamp_keep_alive`: a bare Ollama tag is local, and a
    /// local model must be asked to stay resident. Without this the field went
    /// nowhere at all — `keep_alive` appeared in no request this app made — and
    /// Ollama's five-minute default cost ~2s of reload on the next message.
    #[test]
    fn a_local_model_is_asked_to_stay_loaded() {
        let mut body = serde_json::json!({ "model": "gemma4:12b" });
        stamp_keep_alive(&mut body, "gemma4:12b");
        assert_eq!(body["keep_alive"], KEEP_ALIVE);
    }

    /// …and a cloud model is not. `keep_alive` is Ollama's own field; a strict
    /// endpoint may answer 400 to it, and losing a turn to a latency optimisation
    /// would be a poor trade.
    #[test]
    fn a_cloud_model_is_never_sent_an_ollama_field() {
        for id in ["claude-opus-5", "openai:gpt-5.5", "gemini-2.5-flash"] {
            let mut body = serde_json::json!({ "model": id });
            stamp_keep_alive(&mut body, id);
            assert!(body.get("keep_alive").is_none(), "{id} is not local");
        }
    }

    /// An empty id is not a local model, it is a bug upstream. Asserted because
    /// `runs_locally("")` returning true would put the field on every request the
    /// app makes before a model is chosen.
    #[test]
    fn an_empty_model_id_is_not_stamped() {
        let mut body = serde_json::json!({});
        stamp_keep_alive(&mut body, "");
        assert!(body.get("keep_alive").is_none());
    }
