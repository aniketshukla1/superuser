use super::*;

    /// Each of these flags fails silently when missing: no stream, a session saved to
    /// the user's own history, tools beyond read-only, or a permission question that
    /// nobody answers.
    #[test]
    fn the_command_line_keeps_a_delegated_turn_read_only() {
        let a = argv("sonnet");
        let at = |flag: &str| a.iter().position(|x| x == flag).unwrap_or_else(|| panic!("missing {flag} in {a:?}"));
        at("--print");
        assert_eq!(a[at("--output-format") + 1], "stream-json");
        at("--verbose");
        at("--no-session-persistence");
        at("--strict-mcp-config");
        assert_eq!(a[at("--model") + 1], "sonnet");
        assert_eq!(a[at("--tools") + 1], "Read,Glob,Grep");
        assert_eq!(a[at("--allowedTools") + 1], "Read,Glob,Grep");
        assert_eq!(a[at("--permission-mode") + 1], "dontAsk");
        for x in &a {
            assert!(!x.contains("Bash") && !x.contains("Edit") && !x.contains("Write"), "{x} widens the tools");
            assert!(!x.contains("bypassPermissions") && !x.starts_with("--dangerously"), "{x} drops the guard");
        }
    }

    /// The shape of a successful turn, in the CLI's stream-json vocabulary.
    const STREAM: &str = r#"{"type":"system","subtype":"init","model":"claude-sonnet-5-5","tools":["Glob","Grep","Read"]}
{"type":"assistant","message":{"content":[{"type":"text","text":"I'll look for them."},{"type":"tool_use","id":"toolu_1","name":"Glob","input":{"pattern":"*.txt"}}]}}
{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"toolu_1","content":"a.txt\nb.txt"}]}}
{"type":"assistant","message":{"content":[{"type":"text","text":"There are 2 text files."}]}}
{"type":"result","subtype":"success","is_error":false,"result":"There are 2 text files.","num_turns":2}"#;

    #[test]
    fn a_turn_reads_as_narration_tools_and_an_answer() {
        let mut ups = vec![];
        let mut done = None;
        for line in STREAM.lines() {
            match parse(line) {
                Some(Event::Said(said)) => ups.extend(said),
                Some(Event::Done { ok, text }) => done = Some((ok, text)),
                _ => {}
            }
        }
        assert_eq!(
            ups,
            vec![
                Update::Text("I'll look for them.".into()),
                Update::Tool { name: "Glob".into(), detail: "*.txt".into() },
                Update::Text("There are 2 text files.".into()),
            ]
        );
        assert_eq!(done, Some((true, "There are 2 text files.".into())));
    }

    /// Captured from a real run with an expired sign-in (2026-10-02). The failure comes
    /// as an assistant message first, and must not be shown as Claude's answer.
    #[test]
    fn an_expired_sign_in_is_a_failure_not_a_reply() {
        let msg = r#"{"type":"assistant","message":{"model":"<synthetic>","content":[{"type":"text","text":"Failed to authenticate: OAuth session expired and could not be refreshed"}]},"error":"authentication_failed","is_api_error_message":true}"#;
        assert_eq!(
            parse(msg),
            Some(Event::Failure("Failed to authenticate: OAuth session expired and could not be refreshed".into()))
        );
        let result = r#"{"type":"result","subtype":"success","is_error":true,"result":"Failed to authenticate: OAuth session expired and could not be refreshed","terminal_reason":"api_error"}"#;
        assert!(matches!(parse(result), Some(Event::Done { ok: false, .. })));
        assert!(explain("Failed to authenticate: OAuth session expired", "", "opus").contains("/login"));
    }

    #[test]
    fn unknown_or_broken_lines_are_skipped() {
        for line in [
            r#"{"type":"system","subtype":"hook_started"}"#,
            r#"{"type":"stream_event","event":{}}"#,
            r#"{"type":"assistant","message":{"content":[{"type":"thinking","thinking":""}]}}"#,
            "not json",
            "",
        ] {
            assert_eq!(parse(line), None, "should have skipped: {line}");
        }
    }

    #[test]
    fn a_failure_names_the_thing_to_fix() {
        assert!(explain("", "Error: model claude-nope not found", "claude-nope").contains("`claude-nope`"));
        assert_eq!(explain("Overloaded", "", "opus"), "Claude Code: Overloaded");
        assert!(!explain("", "", "opus").is_empty());
    }

    #[test]
    fn the_handover_follows_the_system_prompt() {
        let p = flatten("You are terse.", &[], "hi");
        assert!(p.find("delegated agent").unwrap() > p.find("You are terse.").unwrap());
        assert!(p.contains("Read, Glob, Grep"));
        assert!(p.trim_end().ends_with("hi"));
    }
