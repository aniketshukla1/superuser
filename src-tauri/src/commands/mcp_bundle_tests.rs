use super::*;
    use serde_json::json;

    #[test]
    fn the_holdings_block_names_each_app_and_the_wrong_door() {
        let mut b = McpBundle::default();
        b.names = vec!["Notion".into(), "Linear".into()];
        b.registry.insert("notion_search".into(), (0, "search".into()));
        b.registry.insert("notion_create_page".into(), (0, "create_page".into()));
        b.registry.insert("linear_list_issues".into(), (1, "list_issues".into()));
        let with_web = b.holdings_block(true);
        assert!(with_web.contains("Notion (2 tools), Linear (1 tool)"), "{with_web}");
        assert!(with_web.contains("Don't fetch or open their websites"), "{with_web}");
        assert!(with_web.contains("name what's missing"), "{with_web}");
        assert!(!b.holdings_block(false).contains("websites"), "no web tools, nothing to warn about");
    }

    /// A minimal MCP server over stdio, so the agent-facing wiring is exercised
    /// against a real handshake rather than a hand-built stub.
    fn fake_server() -> serde_json::Value {
        let script = r#"
while IFS= read -r line; do
  id=$(printf '%s' "$line" | sed -n 's/.*"id":\([0-9]*\).*/\1/p')
  case "$line" in
    *'"initialize"'*)
      printf '{"jsonrpc":"2.0","id":%s,"result":{"serverInfo":{"name":"fake"}}}\n' "$id" ;;
    *'"tools/list"'*)
      printf '{"jsonrpc":"2.0","id":%s,"result":{"tools":[{"name":"ping","description":"say pong","inputSchema":{"type":"object","properties":{}}}]}}\n' "$id" ;;
  esac
done
"#;
        json!({ "command": "sh", "args": ["-c", script] })
    }

    fn entry(name: &str) -> crate::mcp::registry::McpEntry {
        crate::mcp::registry::McpEntry {
            name: name.into(),
            config: fake_server(),
            enabled: true,
            requires_approval: true, tools_fingerprint: String::new(),
        }
    }

    #[tokio::test]
    async fn a_registered_stdio_server_reaches_the_model_as_a_callable_tool() {
        // The gap this closes: a server the user added was listable and toggleable,
        // but its tools never appeared in the table the model is offered, so the
        // agent had no way to call it.
        let http = crate::core::egress::client("mcp");
        let e = entry("notes");
        let mut link = crate::mcp::McpLink::open_entry(&e, &http).await.expect("stdio open");
        let tools = link.list_tools(&http).await.expect("tools/list");

        let mut b = McpBundle::default();
        absorb_mcp_tools(&mut b, &e.name, &e.name, link, &tools, e.requires_approval);

        // Namespaced by server, so two servers offering `search` stay distinct.
        assert!(b.registry.contains_key("notes_ping"), "registry: {:?}", b.registry.keys());
        let (idx, raw) = b.registry["notes_ping"].clone();
        assert_eq!(raw, "ping", "the server's own tool name must survive for the call");
        assert_eq!(idx, 0);

        // Both provider wire formats carry it, since either model may drive the turn.
        assert_eq!(b.claude_tools.len(), 1);
        assert_eq!(b.claude_tools[0]["name"], "notes_ping");
        assert_eq!(b.ollama_tools[0]["function"]["name"], "notes_ping");
        // A schema always goes out, even when the server sent none.
        assert!(b.claude_tools[0]["input_schema"].is_object());
        assert_eq!(b.names, vec!["notes".to_string()]);
        assert!(b.suspect_tools.is_empty());
    }

    #[tokio::test]
    async fn a_server_offering_nothing_is_not_announced_to_the_model() {
        let mut b = McpBundle::default();
        let http = crate::core::egress::client("mcp");
        let e = entry("empty");
        let link = crate::mcp::McpLink::open_entry(&e, &http).await.expect("stdio open");
        absorb_mcp_tools(&mut b, &e.name, &e.name, link, &[], e.requires_approval);
        assert!(b.names.is_empty(), "a server with no tools should not be listed");
        assert!(b.sessions.is_empty(), "and its session should not be held open");
    }

    #[tokio::test]
    async fn two_servers_keep_their_tools_apart() {
        let http = crate::core::egress::client("mcp");
        let mut b = McpBundle::default();
        for name in ["alpha", "beta"] {
            let e = entry(name);
            let mut link = crate::mcp::McpLink::open_entry(&e, &http).await.expect("stdio open");
            let tools = link.list_tools(&http).await.expect("tools/list");
            absorb_mcp_tools(&mut b, &e.name, &e.name, link, &tools, e.requires_approval);
        }
        // Same underlying tool name on both, distinct entries in the table, and each
        // routed back to its OWN session.
        assert_eq!(b.registry["alpha_ping"].0, 0);
        assert_eq!(b.registry["beta_ping"].0, 1);
        assert_eq!(b.claude_tools.len(), 2);
    }

    #[tokio::test]
    async fn a_server_marked_ask_is_not_waved_through_by_the_name_based_guess() {
        // `ping` reads as harmless, so the risk heuristic would allow it outright.
        // The user's own flag on the server has to survive that.
        use crate::core::inbox::{decide, Decision};
        let http = crate::core::egress::client("mcp");
        let e = entry("notes"); // requires_approval: true
        let mut link = crate::mcp::McpLink::open_entry(&e, &http).await.expect("stdio open");
        let tools = link.list_tools(&http).await.expect("tools/list");
        let mut b = McpBundle::default();
        absorb_mcp_tools(&mut b, &e.name, &e.name, link, &tools, e.requires_approval);

        let (idx, _raw) = b.registry["notes_ping"].clone();
        // Precondition: a read-ish tool name is waved straight through by the guess,
        // so the flag is the only thing that could hold such a call back.
        assert_eq!(
            decide("notes_search", "notes", mcp_risk("notes_search"), "interactive", &[], false),
            Decision::Allow,
            "precondition: the name-based guess alone allows a read-looking tool"
        );
        assert!(b.always_ask.contains(&idx), "the server's flag must reach the gate");
    }

    #[tokio::test]
    async fn a_server_left_unmarked_keeps_the_decision_the_risk_guess_made() {
        let http = crate::core::egress::client("mcp");
        let mut e = entry("open");
        e.requires_approval = false;
        let mut link = crate::mcp::McpLink::open_entry(&e, &http).await.expect("stdio open");
        let tools = link.list_tools(&http).await.expect("tools/list");
        let mut b = McpBundle::default();
        absorb_mcp_tools(&mut b, &e.name, &e.name, link, &tools, e.requires_approval);
        assert!(b.always_ask.is_empty());
    }

    #[tokio::test]
    async fn a_server_with_no_way_to_reach_it_is_refused_rather_than_guessed_at() {
        let http = crate::core::egress::client("mcp");
        let e = crate::mcp::registry::McpEntry {
            name: "hollow".into(),
            config: json!({ "type": "http" }),
            enabled: true,
            requires_approval: true, tools_fingerprint: String::new(),
        };
        match crate::mcp::McpLink::open_entry(&e, &http).await {
            Ok(_) => panic!("a config with neither url nor command should not open"),
            Err(e) => assert!(e.contains("neither"), "got: {e}"),
        }
    }
