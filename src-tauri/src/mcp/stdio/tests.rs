use super::*;

    #[test]
    fn a_command_line_is_quoted_so_a_path_with_a_space_survives() {
        let c = json!({ "command": "/opt/my tools/mcp", "args": ["--root", "/Users/a b/x"] });
        assert_eq!(command_line(&c).unwrap(), "'/opt/my tools/mcp' '--root' '/Users/a b/x'");
    }

    #[tokio::test]
    async fn an_apostrophe_cannot_break_out_of_the_quoting() {
        // The command line is handed to `sh -lc`, so an argument carrying a quote
        // and a semicolon must arrive as ONE literal argument rather than as a
        // second command. Asserting the quoted string alone would only restate the
        // implementation, so run it through the same shell and read back what the
        // program actually received.
        let evil = "it's; echo PWNED";
        let line = command_line(&json!({ "command": "echo", "args": [evil] })).unwrap();
        let out = Command::new("sh")
            .arg("-lc")
            .arg(format!("exec {line}"))
            .output()
            .await
            .expect("shell should run");
        let got = String::from_utf8_lossy(&out.stdout).trim().to_string();
        assert_eq!(got, evil, "the argument was re-interpreted by the shell");
        assert!(!got.contains("PWNED\n"), "a second command ran");
    }

    #[test]
    fn a_config_with_no_command_is_not_a_stdio_server() {
        assert!(command_line(&json!({ "type": "http", "url": "https://e/mcp" })).is_none());
        assert!(command_line(&json!({ "command": "   " })).is_none());
    }

    #[test]
    fn args_are_optional() {
        assert_eq!(command_line(&json!({ "command": "npx" })).unwrap(), "'npx'");
    }

    #[tokio::test]
    async fn a_server_that_is_not_there_reports_it_instead_of_hanging() {
        let c = json!({ "command": "definitely-not-a-real-binary-9f3a", "args": [] });
        match StdioSession::open(&c).await {
            Ok(_) => panic!("a missing binary should not open a session"),
            Err(e) => assert!(!e.is_empty()),
        }
    }

    #[tokio::test]
    async fn a_program_that_never_speaks_mcp_fails_rather_than_waiting_forever() {
        // `true` exits immediately without answering: the session must report the
        // exit, not sit on the read until the timeout.
        match StdioSession::open(&json!({ "command": "true" })).await {
            Ok(_) => panic!("`true` speaks no MCP; opening should fail"),
            Err(e) => assert!(e.contains("exited") || e.contains("closed"), "got: {e}"),
        }
    }

    /// A minimal MCP server in shell: echoes back the id it was sent, so the test
    /// exercises the real request/response pairing rather than a fixed sequence.
    fn fake_server() -> Value {
        let script = r#"
while IFS= read -r line; do
  id=$(printf '%s' "$line" | sed -n 's/.*"id":\([0-9]*\).*/\1/p')
  case "$line" in
    *'"initialize"'*)
      printf '{"jsonrpc":"2.0","id":%s,"result":{"serverInfo":{"name":"fake-stdio"}}}\n' "$id" ;;
    *'"tools/list"'*)
      printf '{"jsonrpc":"2.0","id":%s,"result":{"tools":[{"name":"ping","description":"say pong","inputSchema":{"type":"object","properties":{}}}]}}\n' "$id" ;;
    *'"tools/call"'*)
      printf '{"jsonrpc":"2.0","id":%s,"result":{"content":[{"type":"text","text":"pong"}]}}\n' "$id" ;;
  esac
done
"#;
        json!({ "command": "sh", "args": ["-c", script] })
    }

    #[tokio::test]
    async fn a_stdio_server_can_be_opened_listed_and_called() {
        // The whole point of the transport: a server we launch ourselves answers the
        // same three questions an HTTP one does.
        let mut s = StdioSession::open(&fake_server()).await.expect("handshake");
        assert_eq!(s.server_name(), Some("fake-stdio"));

        let tools = s.list_tools().await.expect("tools/list");
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["name"], "ping");

        let out = s.call_tool("ping", json!({})).await.expect("tools/call");
        assert_eq!(out["content"][0]["text"], "pong");
    }

    #[tokio::test]
    async fn several_calls_in_a_row_stay_paired_with_their_own_replies() {
        // Each request carries a fresh id; if the pairing drifted, the second call
        // would return the first one's reply.
        let mut s = StdioSession::open(&fake_server()).await.expect("handshake");
        for _ in 0..3 {
            let out = s.call_tool("ping", json!({})).await.expect("tools/call");
            assert_eq!(out["content"][0]["text"], "pong");
        }
    }

    #[tokio::test]
    async fn a_reply_is_matched_by_id_past_the_noise_a_server_prints_first() {
        // A server that prints a banner, then a notification, then the real reply.
        // Taking "the next line" would read the banner as the handshake.
        let script = r#"read line; echo 'starting up...'; echo '{"jsonrpc":"2.0","method":"notifications/x"}'; echo '{"jsonrpc":"2.0","id":1,"result":{"serverInfo":{"name":"noisy"}}}'; while read l; do :; done"#;
        let c = json!({ "command": "sh", "args": ["-c", script] });
        let s = StdioSession::open(&c).await.expect("handshake should survive the noise");
        assert_eq!(s.server_name(), Some("noisy"));
    }
