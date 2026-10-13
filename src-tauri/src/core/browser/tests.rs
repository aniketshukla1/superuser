use super::*;

    #[test]
    fn a_bare_host_becomes_https() {
        assert_eq!(normalize_url("example.com"), "https://example.com");
        assert_eq!(normalize_url("localhost:3000"), "https://localhost:3000");
    }

    #[test]
    fn an_explicit_scheme_is_left_alone() {
        assert_eq!(normalize_url("http://example.com/x"), "http://example.com/x");
        assert_eq!(normalize_url("https://a.b/c?d=e"), "https://a.b/c?d=e");
    }

    #[test]
    fn prose_becomes_a_search_rather_than_a_broken_host() {
        assert_eq!(normalize_url("who owns the moon"), "https://duckduckgo.com/?q=who+owns+the+moon");
        // No dot, no scheme — a word, not a host.
        assert!(normalize_url("bananas").starts_with("https://duckduckgo.com/?q="));
    }

    #[test]
    fn empty_input_is_a_blank_page_not_a_search_for_nothing() {
        assert_eq!(normalize_url("   "), "about:blank");
    }

    #[test]
    fn a_workspace_with_no_launch_config_simply_offers_nothing() {
        let dir = std::env::temp_dir().join(format!("su-browser-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(dev_servers(&dir), Vec::new());
    }

    #[test]
    fn a_launch_config_becomes_a_runnable_command() {
        let dir = std::env::temp_dir().join(format!("su-browser-cfg-{}", std::process::id()));
        std::fs::create_dir_all(dir.join(".claude")).unwrap();
        std::fs::write(
            dir.join(".claude/launch.json"),
            r#"{"configurations":[
                {"name":"web","runtimeExecutable":"npm","runtimeArgs":["run","dev"],"port":1421},
                {"name":"attach-only","port":9000}
            ]}"#,
        )
        .unwrap();
        let got = dev_servers(&dir);
        // The attach-only entry has nothing to run, so it isn't offered.
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].name, "web");
        assert_eq!(got[0].command, "npm run dev");
        assert_eq!(got[0].url, "http://localhost:1421");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn an_argument_with_spaces_survives_as_one_word() {
        // This repo's own launch.json passes a `sh -c "…"` script; unquoted it would
        // fall apart into a dozen arguments the moment it was typed into a shell.
        assert_eq!(shell_quote("npm run dev"), "'npm run dev'");
        assert_eq!(shell_quote("run"), "run");
        assert_eq!(shell_quote("it's"), r#"'it'\''s'"#);
    }

    #[test]
    fn a_malformed_launch_config_is_ignored_rather_than_fatal() {
        let dir = std::env::temp_dir().join(format!("su-browser-bad-{}", std::process::id()));
        std::fs::create_dir_all(dir.join(".claude")).unwrap();
        std::fs::write(dir.join(".claude/launch.json"), "{ not json at all").unwrap();
        assert_eq!(dev_servers(&dir), Vec::new());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn local_file_and_script_schemes_are_refused() {
        assert!(is_blocked_scheme("file:///etc/passwd"));
        assert!(is_blocked_scheme("JavaScript:alert(1)"));
        assert!(is_blocked_scheme("data:text/html,<h1>x"));
        assert!(!is_blocked_scheme("https://example.com"));
    }
