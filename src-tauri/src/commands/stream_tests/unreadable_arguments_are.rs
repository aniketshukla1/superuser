use super::*;

    #[test]
    pub(super) fn an_mcp_tool_is_serial_whatever_its_risk_class_says() {
        use crate::core::agent::{RiskClass, ToolSpec};
        // Calling one needs `&mut McpBundle` for its session state — and the risk
        // class here is a NAME-BASED GUESS about somebody else's vocabulary that the
        // user can override in Settings. Neither is a claim about thread-safety.
        let mut mcp = McpBundle::default();
        mcp.registry.insert("linear_search".into(), (0, "search".into()));
        let specs = vec![ToolSpec {
            name: "linear_search".to_string(),
            description: String::new(),
            capability: String::new(),
            risk: RiskClass::Read,
            schema: serde_json::json!({}),
        }];
        assert!(
            !parallel_safe("linear_search", &specs, &mcp),
            "an MCP tool must stay serial even when its risk reads as Read"
        );
    }

    #[test]
    pub(super) fn a_message_records_usage_only_when_there_is_some() {
        use crate::core::sessions::{Message, TokenUsage};
        assert!(Message::new("assistant", "x").with_usage(None).usage.is_none());
        assert!(Message::new("assistant", "x")
            .with_usage(Some(TokenUsage::default()))
            .usage
            .is_none());
        let m = Message::new("assistant", "x").with_usage(Some(TokenUsage { input: 3, output: 4, ..Default::default() }));
        assert_eq!(m.usage.unwrap().total(), 7);
    }

    #[test]
    pub(super) fn openai_style_reasoning_is_read_under_either_name() {
        // Providers disagree on the field name; both appear in the wild.
        let mut a = StreamAccum::default();
        assert_eq!(
            a.push_line(r#"data: {"choices":[{"delta":{"reasoning_content":"weighing"}}]}"#, true).thinking.as_deref(),
            Some("weighing")
        );
        let mut b = StreamAccum::default();
        assert_eq!(
            b.push_line(r#"data: {"choices":[{"delta":{"reasoning":"weighing"}}]}"#, true).thinking.as_deref(),
            Some("weighing")
        );
        // And a model that reports none still streams its answer normally.
        let mut c = StreamAccum::default();
        let d = c.push_line(r#"data: {"choices":[{"delta":{"content":"hello"}}]}"#, true);
        assert_eq!(d.text.as_deref(), Some("hello"));
        assert_eq!(d.thinking, None);
    }

    #[test]
    pub(super) fn ollama_tool_calls_survive_streaming() {
        let (_, msg, _) = replay(
            &[
                r#"{"message":{"role":"assistant","content":"","tool_calls":[{"function":{"name":"write_file","arguments":{"path":"a.txt","content":"OK"}}}]},"done":false}"#,
                r#"{"message":{"role":"assistant","content":""},"done":true}"#,
            ],
            false,
        );
        let call = &msg["tool_calls"][0]["function"];
        assert_eq!(call["name"], "write_file");
        assert_eq!(call["arguments"]["path"], "a.txt");
    }

    #[test]
    pub(super) fn openai_sse_reassembles_arguments_split_across_chunks() {
        // The sharp edge: `arguments` is only valid JSON once the fragments are joined.
        let (shown, msg, done) = replay(
            &[
                r#"data: {"choices":[{"delta":{"content":"On it."}}]}"#,
                "",
                r#"data: {"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_1","function":{"name":"write_file","arguments":"{\"path\":"}}]}}]}"#,
                r#"data: {"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"\"a.txt\"}"}}]}}]}"#,
                "data: [DONE]",
            ],
            true,
        );
        assert_eq!(shown, "On it.");
        assert!(done);
        let call = &msg["tool_calls"][0];
        assert_eq!(call["id"], "call_1");
        assert_eq!(call["function"]["name"], "write_file");
        // The agent loop parses this string — it has to be complete, valid JSON.
        let args: serde_json::Value =
            serde_json::from_str(call["function"]["arguments"].as_str().unwrap()).unwrap();
        assert_eq!(args, json!({ "path": "a.txt" }));
    }

    #[test]
    pub(super) fn two_parallel_calls_stay_separate() {
        let (_, msg, _) = replay(
            &[
                r#"data: {"choices":[{"delta":{"tool_calls":[{"index":0,"id":"a","function":{"name":"read_file","arguments":"{}"}}]}}]}"#,
                r#"data: {"choices":[{"delta":{"tool_calls":[{"index":1,"id":"b","function":{"name":"grep","arguments":"{}"}}]}}]}"#,
                "data: [DONE]",
            ],
            true,
        );
        assert_eq!(msg["tool_calls"].as_array().unwrap().len(), 2);
        assert_eq!(msg["tool_calls"][0]["function"]["name"], "read_file");
        assert_eq!(msg["tool_calls"][1]["function"]["name"], "grep");
    }

    /// H3: arguments that do not parse come back as the call's own result.
    #[test]
    pub(super) fn unreadable_arguments_are_reported_rather_than_run_as_empty() {
        // The shape an OpenAI-compatible model sends: arguments as a JSON string.
        let (args, bad) = read_arguments(&serde_json::json!(r#"{"path":"a.txt"}"#));
        assert_eq!(args["path"], "a.txt");
        assert!(bad.is_none(), "valid JSON must not be reported as unreadable");

        // Truncated mid-argument — what a model hitting its output cap actually sends.
        let (args, bad) = read_arguments(&serde_json::json!(r#"{"path":"a.tx"#));
        let why = bad.expect("a truncated argument string must be reported");
        assert!(why.starts_with("error:"), "must read as a tool error: {why}");
        assert!(why.contains("JSON"), "must say what was wrong: {why}");
        // Still an object, so the event the UI renders has a shape to render.
        assert!(args.is_object());
        // And emphatically not a call with no arguments: the whole bug was that
        // `write_file` then ran with no path and returned an error about something
        // the model had not done.
        assert!(args.get("path").is_none());

        // Claude's format hands over an object the server already parsed.
        let (args, bad) = read_arguments(&serde_json::json!({"path": "b.txt"}));
        assert_eq!(args["path"], "b.txt");
        assert!(bad.is_none());
    }

    #[test]
    pub(super) fn junk_and_keepalives_are_ignored_rather_than_ending_the_turn() {
        let (shown, msg, done) = replay(
            &["", "   ", ": keep-alive", "data: not json", r#"data: {"choices":[{"delta":{"content":"hi"}}]}"#],
            true,
        );
        assert_eq!(shown, "hi");
        assert_eq!(msg["content"], "hi");
        assert!(!done, "no [DONE] seen, so the stream isn't finished");
    }
