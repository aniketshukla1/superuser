use super::*;

    /// Every flag here failed once, or would fail silently in a way nothing else
    /// catches. `--skip-git-repo-check` is the one that shipped missing: the session
    /// directory is not a Git repo, so Codex refused every turn with "Not inside a
    /// trusted directory" — a message that names neither the model nor the prompt,
    /// and sends you looking in the wrong place.
    #[test]
    fn the_command_line_carries_what_codex_needs_to_start() {
        let a = argv("gpt-5.6-sol", Path::new("/tmp/session-1"));

        let i = |flag: &str| a.iter().position(|x| x == flag).unwrap_or_else(|| panic!("missing {flag} in {a:?}"));
        i("--skip-git-repo-check");
        // Without this the CLI prints prose at the end instead of events as they
        // happen, and the live panel has nothing to render until the turn is over.
        i("--json");

        // read-only is the value, not just the flag: the CLI's own options reach
        // `danger-full-access`, and a prompt built from chat content must not.
        assert_eq!(a[i("--sandbox") + 1], "read-only");

        // Waiving the approval *prompt* — there is no one on stdin to answer it —
        // must never be done by widening the sandbox, which is what the CLI's own
        // two options for this would do.
        assert_eq!(a[i("-c") + 1], "approval_policy=\"never\"");
        assert!(!a.iter().any(|x| x == "--approve-for-me"), "that escalates to workspace-write");
        assert!(!a.iter().any(|x| x.starts_with("--dangerously-")), "that drops the sandbox");
        assert_eq!(a[i("-m") + 1], "gpt-5.6-sol");
        assert_eq!(a[i("-C") + 1], "/tmp/session-1");

        // `-` last: it means "prompt on stdin", and anything after it would be read
        // as the prompt instead.
        assert_eq!(a.last().map(String::as_str), Some("-"));
        assert_eq!(a.first().map(String::as_str), Some("exec"));
    }

    /// Captured from a real `codex exec --json` run. These four lines are the whole
    /// stream for a simple turn, and the shapes the parser has to survive.
    const REAL_STREAM: &str = r#"{"type":"thread.started","thread_id":"01a02599"}
{"type":"turn.started"}
{"type":"item.started"}
{"type":"item.completed","item":{"id":"item_0","type":"agent_message","text":"I'll inspect that directory."}}
{"type":"item.completed","item":{"id":"item_1","type":"command_execution","command":"/bin/zsh -lc \"find . -name '*.rs'\"","aggregated_output":"a.rs\nb.rs"}}
{"type":"item.completed","item":{"id":"item_2","type":"agent_message","text":"There are 2 files."}}
{"type":"turn.completed","usage":{"input_tokens":16111,"output_tokens":13}}"#;

    /// What a person watching wants is the narration and the commands — not the
    /// stream's own bookkeeping. Every `thread.started`/`turn.started`/`item.started`
    /// forwarded would be a line of noise in the panel for no information.
    #[test]
    fn only_the_events_worth_watching_come_through() {
        let ups: Vec<Update> = REAL_STREAM.lines().filter_map(parse_event).collect();
        assert_eq!(
            ups,
            vec![
                Update::Text("I'll inspect that directory.".into()),
                Update::Command("/bin/zsh -lc \"find . -name '*.rs'\"".into()),
                Update::Text("There are 2 files.".into()),
            ]
        );
    }

    /// The stream is an unversioned CLI contract. A new event type in some future
    /// release must cost the user nothing — the parser skips what it doesn't know
    /// rather than failing the turn on it.
    #[test]
    fn an_unknown_or_broken_line_is_skipped_not_fatal() {
        for line in [
            r#"{"type":"item.completed","item":{"type":"something_new","text":"?"}}"#,
            r#"{"type":"item.completed","item":{"type":"agent_message"}}"#, // no text
            r#"{"type":"item.completed","item":{"type":"agent_message","text":"   "}}"#, // blank
            "not json at all",
            "",
        ] {
            assert_eq!(parse_event(line), None, "should have skipped: {line}");
        }
    }

    /// A history flattened without role labels reads as one voice, and Codex answers
    /// the wrong turn — so the labels, and the separator after the instructions, are
    /// the part worth pinning.
    #[test]
    fn a_conversation_survives_being_flattened() {
        let h = vec![
            ("user".to_string(), "what is 2+2".to_string()),
            ("assistant".to_string(), "4".to_string()),
            ("user".to_string(), "   ".to_string()), // blank turns are noise
        ];
        let p = flatten("You are terse.", &h, "and 3+3?");
        assert!(p.starts_with("You are terse."), "instructions lead: {p:?}");
        // The correction has to come *after* the app's instructions, or the thing it
        // is correcting is the thing that gets the last word.
        assert!(
            p.find("delegated agent").unwrap() > p.find("You are terse.").unwrap(),
            "the handover note must follow the system prompt it overrides"
        );
        assert!(p.contains("Do not ask for approval"), "the turn dead-ends without this");
        assert!(p.contains("User: what is 2+2"));
        assert!(p.contains("Assistant: 4"));
        assert!(p.trim_end().ends_with("and 3+3?"), "the live question lands last: {p:?}");
        assert!(!p.contains("User: \n"), "a blank turn should be dropped, not labelled");
    }

    /// With no instructions there is nothing to separate, so the rule must not leak
    /// into the prompt as a stray divider.
    #[test]
    fn an_empty_system_prompt_adds_no_divider() {
        assert_eq!(flatten("", &[], "hello"), "hello");
        assert_eq!(flatten("   ", &[], "hello"), "hello");
    }

    /// The message a user can act on is the one naming the model or the login; the
    /// generic fallback must still say something rather than echo a warning line.
    #[test]
    fn a_failure_says_which_thing_to_fix() {
        let unsupported = "warning: Model metadata not found\nERROR: {\"message\":\"The 'gpt-5.3-codex' model is not supported when using Codex with a ChatGPT account.\"}";
        assert!(explain(unsupported, "gpt-5.3-codex").contains("doesn't include `gpt-5.3-codex`"));

        assert!(explain("Your access token could not be refreshed. Please log out and sign in again.", "m").contains("codex login"));

        // Warnings are not diagnoses: the real last line wins over them.
        let noisy = "warning: something cosmetic\nconnection reset by peer\nwarning: another";
        assert_eq!(explain(noisy, "m"), "Codex: connection reset by peer");
        assert!(!explain("", "m").is_empty());
    }

    /// A turn that starts and then fails says why on STDOUT, and stderr stays empty
    /// — so `explain` has nothing and the reason has to come off the JSONL stream.
    /// These are the exact lines `codex exec --json` emitted on a plan that had run
    /// out of credit; the app reported "Codex failed without saying why" over them.
    #[test]
    fn a_mid_turn_failure_is_read_off_the_json_stream() {
        let limit = "You've hit your usage limit. Upgrade to Pro (https://chatgpt.com/explore/pro), \
                     visit https://chatgpt.com/codex/settings/usage to purchase more credits or try again at 2:44 PM.";
        assert_eq!(
            parse_error(&format!("{{\"type\":\"error\",\"message\":\"{limit}\"}}")).as_deref(),
            Some(limit)
        );
        // The same reason arrives again wrapped in `turn.failed`.
        assert_eq!(
            parse_error(&format!("{{\"type\":\"turn.failed\",\"error\":{{\"message\":\"{limit}\"}}}}")).as_deref(),
            Some(limit)
        );
        // Everything else on the stream is not a failure.
        assert_eq!(parse_error("{\"type\":\"turn.started\"}"), None);
        assert_eq!(parse_error("{\"type\":\"thread.started\",\"thread_id\":\"01a0\"}"), None);
        assert_eq!(parse_error("{\"type\":\"item.completed\",\"item\":{\"type\":\"agent_message\",\"text\":\"hi\"}}"), None);
        assert_eq!(parse_error("not json"), None);
        assert_eq!(parse_error("{\"type\":\"error\",\"message\":\"  \"}"), None);
        // And a failure line is never mistaken for narration.
        assert_eq!(parse_event(&format!("{{\"type\":\"error\",\"message\":\"{limit}\"}}")), None);
    }
