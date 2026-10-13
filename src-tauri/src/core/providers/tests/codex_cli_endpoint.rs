use super::*;

/// A key saved in this app must beat a CLI login that happens to be on the machine.
    /// Typing one in is a deliberate choice; finding somebody's CLI session and using it
    /// instead would quietly send their turns somewhere they did not pick.
    #[test]
    pub(super) fn our_own_key_wins_over_the_cli_login() {
        // Only the precedence is asserted: reading the CLI file needs a real HOME with a
        // real auth.json, which is the machine's business and not a unit test's.
        if let Some(k) = crate::core::secrets::get_provider_key("opencode") {
            assert_eq!(key_for("opencode").as_deref(), Some(k.as_str()));
        }
        // A provider with no CLI of its own never consults one, whatever is on disk.
        assert_eq!(key_for("groq"), crate::core::secrets::get_provider_key("groq"));
    }

    /// Every curated OpenCode id must be one a key with no billing can actually run — the
    /// free tier — or one of the two flagships kept for the day billing is added. An id that
    /// is neither is a picker entry that fails at turn time.
    #[test]
    pub(super) fn the_opencode_list_is_free_models_plus_the_two_flagships() {
        let p = find("opencode").expect("opencode provider is registered");
        assert_eq!(p.base_url, "https://opencode.ai/zen/v1", "the OpenAI-compatible surface");
        let (free, paid): (Vec<&&str>, Vec<&&str>) =
            p.models.iter().partition(|m| m.ends_with("-free") || **m == "big-pickle");
        assert!(free.len() >= 5, "the free tier is the part that works today: {free:?}");
        assert_eq!(paid, vec![&"claude-sonnet-5", &"gpt-5.6-sol"], "only the two flagships");
        // Omitted on purpose: mimo-v2.5-free (rate limited) and
        // muse-spark-1.2-contributor-free (500) both failed a live tool-calling check.
        for bad in ["mimo-v2.5-free", "muse-spark-1.2-contributor-free"] {
            assert!(!p.models.contains(&bad), "{bad} failed its check and must not be offered");
        }
    }

    #[test]
    pub(super) fn resolve_prefixed_and_bare() {
        assert_eq!(resolve("openai:gpt-5.5").unwrap().id, "openai");
        assert_eq!(model_name("openai:gpt-5.5"), "gpt-5.5");

        assert_eq!(resolve("claude-opus-4-8").unwrap().id, "anthropic");
        assert_eq!(model_name("claude-opus-4-8"), "claude-opus-4-8");

        assert_eq!(resolve("gpt-5").unwrap().kind, ProviderKind::OpenAiCompatible);
        assert_eq!(resolve("deepseek-chat").unwrap().id, "deepseek");

        // Unrecognized bare id → local Ollama (None).
        assert!(resolve("llama3.2").is_none());
        assert!(resolve("qwen2.5").is_none());
    }

    /// An Ollama tag is `name:tag`, which looks exactly like `provider:model`. Splitting a
    /// local id on the colon made "qwen2.5:7b-instruct" into "7b-instruct" — installed
    /// nowhere — so every tagged local model silently ran as something else.
    #[test]
    pub(super) fn a_custom_provider_on_loopback_counts_as_local() {
        // The case this got wrong: `custom` is registered `local: false` because the
        // provider CLASS is not inherently local, but it is the escape hatch people
        // point at Ollama — and its own registry comment says so. Judging by the class
        // alone called a model on localhost "remote", which is how the contention
        // notice came to be missing from the only setup that needed it.
        assert!(is_loopback("http://localhost:11434/v1"));
        assert!(is_loopback("http://127.0.0.1:1234/v1"));
        assert!(!is_loopback("https://api.anthropic.com"));

        // Ollama's own ids are local whatever the address store says.
        assert!(runs_locally("gemma4:12b"));
        assert!(runs_locally("ollama:llama3"));
        // A cloud provider is not, and an empty id answers nothing rather than "yes".
        assert!(!runs_locally("claude-opus-5"));
        assert!(!runs_locally(""));
    }

    #[test]
    pub(super) fn the_effective_model_is_never_empty() {
        // The whole fix for the model bleed rests on this: `ready` reports
        // `effective_model` for a session that never picked one, and the GUI treats an
        // empty model as "keep whatever the last chat had". If this can return "", the
        // bleed comes straight back through a resolver that looks like it fixed it.
        let mut s = crate::core::settings::Settings::default();
        assert!(!effective_model(&s).is_empty(), "default settings");

        // Cloud off, local-only, cloud model set to nonsense — still a real answer.
        s.local_only = true;
        s.cloud_enabled = false;
        s.cloud_model = String::new();
        assert_eq!(effective_model(&s), s.ollama_model);
        assert!(!effective_model(&s).is_empty());
    }

    #[test]
    pub(super) fn a_local_only_user_never_gets_the_cloud_model_back() {
        // The failure this rule was written for: the label and the turn disagreeing.
        let mut s = crate::core::settings::Settings::default();
        s.local_only = true;
        s.cloud_enabled = true;
        s.cloud_model = "claude-opus-5".into();
        s.ollama_model = "gemma4:12b".into();
        assert_eq!(effective_model(&s), "gemma4:12b", "local_only is a hard switch");
    }

    #[test]
    pub(super) fn ollama_tags_are_not_provider_prefixes() {
        for id in ["qwen2.5:7b-instruct", "llama3.2:latest", "gemma4:12b"] {
            assert!(resolve(id).is_none(), "{id} is a local model, not a cloud provider");
            assert_eq!(model_name(id), id, "{id} must reach Ollama with its tag intact");
        }
        // A real provider prefix is still stripped.
        assert_eq!(model_name("openai:gpt-5.5"), "gpt-5.5");
        assert_eq!(model_name("anthropic:claude-opus-4-8"), "claude-opus-4-8");
    }

    #[test]
    pub(super) fn gemini_uses_the_openai_compatible_path() {
        // Gemini's OpenAI-compatible surface means the assistant tool loop needs no
        // separate client — every non-Anthropic cloud provider goes one way.
        let g = resolve("gemini-2.5-pro").unwrap();
        assert_eq!(g.id, "gemini");
        assert_eq!(g.kind, ProviderKind::OpenAiCompatible);
        assert!(g.base_url.ends_with("/openai"));
        // Codex is the other exception: it is not an HTTP endpoint at all, it is a
        // CLI that runs its own agent, so it has no base url to route to.
        for p in PROVIDERS.iter().filter(|p| !p.local && p.id != "anthropic" && !p.kind.delegates()) {
            assert_eq!(p.kind, ProviderKind::OpenAiCompatible, "{} should route OpenAI-compatibly", p.id);
        }
    }

    /// The override wins, the default fills in, and a blank override is not an
    /// address — a cleared field must restore the vendor default rather than send
    /// the turn to the empty string.
    #[test]
    pub(super) fn override_wins_over_the_registry_default() {
        assert_eq!(resolve_base("openai", None), "https://api.openai.com/v1");
        assert_eq!(
            resolve_base("openai", Some("http://127.0.0.1:10531/v1".into())),
            "http://127.0.0.1:10531/v1"
        );
        for blank in ["", "   "] {
            assert_eq!(
                resolve_base("openai", Some(blank.into())),
                "https://api.openai.com/v1",
                "a blank override is not an address"
            );
        }
        // Trailing slashes are stripped on both paths: `/chat/completions` is
        // appended at the call site, and `…/v1//chat/completions` 404s.
        assert_eq!(resolve_base("openai", Some("http://localhost:8080/v1/".into())), "http://localhost:8080/v1");
    }

    /// `custom` ships with no address on purpose — the address is its whole
    /// configuration — so it must resolve by prefix and report itself unready
    /// until one is set, rather than silently posting to "".
    #[test]
    pub(super) fn custom_provider_has_no_default_address() {
        let c = find("custom").unwrap();
        assert_eq!(c.kind, ProviderKind::OpenAiCompatible);
        assert!(c.base_url.is_empty());
        assert_eq!(resolve_base("custom", None), "");
        assert_eq!(resolve("custom:gpt-5.5").unwrap().id, "custom");
        assert_eq!(model_name("custom:gpt-5.5"), "gpt-5.5");
    }

    /// Codex is reached by running a CLI, not by posting to a URL, so the two
    /// things that would quietly break it are an endpoint being invented for it and
    /// a model id that a ChatGPT plan won't serve.
    #[test]
    pub(super) fn codex_is_a_cli_not_an_endpoint() {
        let c = find("codex").unwrap();
        assert_eq!(c.kind, ProviderKind::CodexCli);
        assert!(c.base_url.is_empty(), "there is no endpoint to post to");
        assert!(c.key_hint.is_empty(), "there is no key to paste");
        assert_eq!(resolve_base("codex", None), "");

        // Selected as `codex:<model>`; the bare name must not be sent on the wire.
        assert_eq!(resolve("codex:gpt-5.6-sol").unwrap().id, "codex");
        assert_eq!(model_name("codex:gpt-5.6-sol"), "gpt-5.6-sol");

        // Only models a ChatGPT plan actually serves. `gpt-5.6-pro` and the
        // `-codex` ids answer "not supported when using Codex with a ChatGPT
        // account", so listing them would put ids in the picker that always fail.
        assert!(!c.models.is_empty());
        for m in c.models {
            assert!(m.starts_with("gpt-5.6-"), "{m} is not a plan model");
            assert!(!m.ends_with("-pro"), "{m} needs API billing, not a plan");
        }
    }
