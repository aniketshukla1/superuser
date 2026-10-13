use super::*;

    /// Claude Code is a CLI like Codex: no endpoint, no key, selected as
    /// `claude-code:<alias>`, and the alias is what reaches the CLI's `--model`.
    #[test]
    pub(super) fn claude_code_is_a_cli_not_an_endpoint() {
        let c = find("claude-code").unwrap();
        assert_eq!(c.kind, ProviderKind::ClaudeCli);
        assert!(c.kind.delegates() && ProviderKind::CodexCli.delegates());
        assert!(!ProviderKind::Anthropic.delegates());
        assert!(c.base_url.is_empty() && c.key_hint.is_empty());
        assert_eq!(resolve("claude-code:opus").unwrap().id, "claude-code");
        assert_eq!(model_name("claude-code:opus"), "opus");
        // A bare Claude id still means the API provider, not the CLI.
        assert_eq!(resolve("claude-opus-4-8").unwrap().id, "anthropic");
    }

    /// The one rule here that has a wrong answer worse than an error: a remote
    /// endpoint with no key must refuse, not post the conversation unauthenticated.
    #[test]
    pub(super) fn a_missing_key_only_passes_on_this_machine() {
        // On this machine, no key is normal — the proxy the user is running has none.
        let (url, key) = endpoint("http://127.0.0.1:10531/v1", None, "Custom").unwrap();
        assert_eq!(url, "http://127.0.0.1:10531/v1/chat/completions");
        assert_eq!(key, None);

        // Anywhere else it is a refusal, not a keyless request.
        let err = endpoint("https://api.openai.com/v1", None, "OpenAI").unwrap_err();
        assert!(err.contains("Add an API key"), "got: {err}");
        // Including hosts that merely look local.
        assert!(endpoint("http://localhost.evil.com/v1", None, "Custom").is_err());

        // A key is passed through wherever it points.
        let (_, key) = endpoint("https://api.groq.com/openai/v1", Some("gsk_x".into()), "Groq").unwrap();
        assert_eq!(key.as_deref(), Some("gsk_x"));

        // No address at all names the field to fix rather than posting to "".
        let err = endpoint("", None, "Custom").unwrap_err();
        assert!(err.contains("base URL"), "got: {err}");
    }

    /// Both rejections here stand in for a failure that surfaces far from its cause
    /// — one inside a turn, one as a bare 404 — so they have to be caught at the
    /// moment the URL is typed, and blank has to stay a clear rather than an error.
    #[test]
    pub(super) fn a_typed_endpoint_is_checked_where_it_is_typed() {
        assert_eq!(validate_base_url("https://api.groq.com/openai/v1/").unwrap(), "https://api.groq.com/openai/v1");
        assert_eq!(validate_base_url("http://127.0.0.1:10531/v1").unwrap(), "http://127.0.0.1:10531/v1");
        // Blank clears the override; that is not a failure.
        assert_eq!(validate_base_url("   ").unwrap(), "");

        assert!(validate_base_url("api.openai.com/v1").is_err(), "a schemeless URL fails inside a turn, not here");
        assert!(validate_base_url("localhost:10531/v1").is_err());
        assert!(
            validate_base_url("http://127.0.0.1:10531/v1/chat/completions").is_err(),
            "the path is appended at call time, so including it doubles up and 404s"
        );
    }

    /// A custom endpoint is unpickable without a model list, so the override has to
    /// survive the untidy input a human types: spaces after commas, a trailing one.
    #[test]
    pub(super) fn a_custom_endpoint_can_declare_its_own_models() {
        assert_eq!(resolve_models("custom", None), Vec::<String>::new());
        assert_eq!(
            resolve_models("custom", Some(" gpt-5.5 , gpt-5-mini ,".into())),
            vec!["gpt-5.5", "gpt-5-mini"]
        );
        // Blank clears rather than emptying: a provider with a curated list keeps it.
        assert_eq!(resolve_models("openai", Some("  ,, ".into())), vec!["gpt-5.5", "gpt-5", "gpt-5-mini"]);
        assert_eq!(resolve_models("openai", None), vec!["gpt-5.5", "gpt-5", "gpt-5-mini"]);
    }

    /// Whether a key is required hangs on this, so a host that is not this machine
    /// must never read as loopback — `notlocalhost.com` is the shape that would
    /// turn a missing key into an unauthenticated request to a stranger.
    #[test]
    pub(super) fn loopback_is_only_this_machine() {
        for yes in [
            "http://127.0.0.1:10531/v1",
            "http://localhost:8080/v1",
            "https://localhost/v1",
            "http://[::1]:1234/v1",
            "http://0.0.0.0:11434/v1",
        ] {
            assert!(is_loopback(yes), "{yes} is this machine");
        }
        for no in [
            "https://api.openai.com/v1",
            "http://notlocalhost.com/v1",
            "https://localhost.evil.com/v1",
            "http://127.0.0.1.evil.com/v1",
            "",
        ] {
            assert!(!is_loopback(no), "{no} is NOT this machine");
        }
    }

    #[test]
    pub(super) fn registry_has_core_providers() {
        for id in ["ollama", "anthropic", "openai", "gemini", "deepseek", "xai"] {
            assert!(find(id).is_some(), "missing provider {id}");
        }
        // OpenAI-compatible providers carry a base url; Ollama/Anthropic don't need one for this path.
        let openai = find("openai").unwrap();
        assert!(openai.base_url.ends_with("/v1"));
        assert!(!openai.models.is_empty());
    }
