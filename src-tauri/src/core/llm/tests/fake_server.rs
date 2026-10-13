use super::*;

    #[test]
    pub(super) fn tools_go_on_the_wire_in_a_fixed_order() {
        // `tools` is FIRST in the cache hierarchy, so drift there invalidates
        // everything after it. The order out of capability expansion is stable today
        // and is not guaranteed to be.
        let specs = crate::core::agent::expand(
            &["files".to_string(), "git".to_string(), "web".to_string()],
            &crate::core::agent::AgentContext {
                workspace: Some(std::path::PathBuf::from("/tmp/ws")),
                roots: vec![std::path::PathBuf::from("/tmp/ws")],
                has_executor: true,
                has_todo: true,
            },
        );
        // Sorted the way `tools_claude` sorts, then checked that the sort is what
        // reaches the wire: `deferred_tools` must preserve it, since it rebuilds the
        // array to add the search tool and the defer markers.
        let mut wire: Vec<Value> = specs
            .iter()
            .map(|t| json!({"name": t.name, "description": t.description, "input_schema": t.schema}))
            .collect();
        wire.sort_by(|a, b| a["name"].as_str().unwrap_or("").cmp(b["name"].as_str().unwrap_or("")));
        let out = LlmRequest::new("claude-opus-5", 4096).tools(wire.clone()).deferred_tools();
        // First is the search tool; the rest keep the sorted order.
        let names: Vec<&str> = out[1..].iter().map(|t| t["name"].as_str().unwrap_or("")).collect();
        let mut sorted = names.clone();
        sorted.sort_unstable();
        assert_eq!(names, sorted, "deferral must not reshuffle the tools array");
    }

    /// A reply as the Messages API streams it: text, a thinking block with its
    /// signature, a tool call whose input arrives in fragments, and the usage split
    /// between the start and the final delta.
    pub(super) const STREAM: &str = concat!(
        "event: message_start\n",
        "data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_1\",\"model\":\"claude-opus-5\",\"usage\":{\"input_tokens\":12,\"cache_read_input_tokens\":900,\"cache_creation_input_tokens\":100,\"output_tokens\":1}}}\n\n",
        "event: content_block_start\n",
        "data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"thinking\",\"thinking\":\"\",\"signature\":\"\"}}\n\n",
        "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"thinking_delta\",\"thinking\":\"Check the \"}}\n\n",
        "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"thinking_delta\",\"thinking\":\"file first.\"}}\n\n",
        "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"signature_delta\",\"signature\":\"EqQBCgIYAh\"}}\n\n",
        "data: {\"type\":\"content_block_stop\",\"index\":0}\n\n",
        "event: ping\n",
        "data: {\"type\":\"ping\"}\n\n",
        "data: {\"type\":\"content_block_start\",\"index\":1,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\n",
        "data: {\"type\":\"content_block_delta\",\"index\":1,\"delta\":{\"type\":\"text_delta\",\"text\":\"Looking \"}}\n\n",
        "data: {\"type\":\"content_block_delta\",\"index\":1,\"delta\":{\"type\":\"text_delta\",\"text\":\"that up.\"}}\n\n",
        "data: {\"type\":\"content_block_stop\",\"index\":1}\n\n",
        "data: {\"type\":\"content_block_start\",\"index\":2,\"content_block\":{\"type\":\"tool_use\",\"id\":\"toolu_1\",\"name\":\"read_file\",\"input\":{}}}\n\n",
        "data: {\"type\":\"content_block_delta\",\"index\":2,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"\"}}\n\n",
        "data: {\"type\":\"content_block_delta\",\"index\":2,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"{\\\"path\\\": \\\"a\"}}\n\n",
        "data: {\"type\":\"content_block_delta\",\"index\":2,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\".md\\\"}\"}}\n\n",
        "data: {\"type\":\"content_block_stop\",\"index\":2}\n\n",
        "data: {\"type\":\"content_block_start\",\"index\":3,\"content_block\":{\"type\":\"tool_use\",\"id\":\"toolu_2\",\"name\":\"git_status\",\"input\":{}}}\n\n",
        "data: {\"type\":\"content_block_stop\",\"index\":3}\n\n",
        "event: message_delta\n",
        "data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"tool_use\",\"stop_sequence\":null},\"usage\":{\"output_tokens\":42}}\n\n",
        "event: message_stop\n",
        "data: {\"type\":\"message_stop\"}\n\n",
    );

    #[test]
    pub(super) fn a_streamed_reply_assembles_into_the_shape_the_loop_always_read() {
        let mut acc = SseAccum::default();
        for line in STREAM.lines() {
            acc.push_line(line);
        }
        assert!(acc.done && acc.started);
        let reply = acc.finish("claude-opus-5");
        assert_eq!(reply.stop_reason, "tool_use");
        assert_eq!(reply.content.len(), 4);
        // The thinking block survives whole, signature included — the API rejects a
        // tool-use turn whose thinking comes back altered.
        assert_eq!(reply.content[0], json!({"type":"thinking","thinking":"Check the file first.","signature":"EqQBCgIYAh"}));
        assert_eq!(reply.text(), "Looking that up.");
        let calls = reply.tool_uses();
        assert_eq!(calls[0]["input"], json!({"path": "a.md"}), "fragments join into one object");
        assert_eq!(calls[1]["input"], json!({}), "no fragments is an empty object, not a missing one");
        let u = reply.usage.expect("usage");
        assert_eq!((u.input, u.output, u.cache_read, u.cache_write), (12, 42, 900, 100));
    }

    #[test]
    pub(super) fn an_error_event_is_an_error_not_an_empty_reply() {
        let mut acc = SseAccum::default();
        acc.push_line("data: {\"type\":\"message_start\",\"message\":{\"usage\":{}}}");
        acc.push_line("data: {\"type\":\"error\",\"error\":{\"type\":\"overloaded_error\",\"message\":\"Overloaded\"}}");
        assert_eq!(acc.error.as_deref(), Some("Overloaded"));
    }

    /// A loopback server that reads one request, hands its body to the test, and then
    /// does whatever `script` says with the socket.
    pub(super) async fn fake_server<F, Fut>(script: F) -> (String, tokio::sync::oneshot::Receiver<String>)
    where
        F: FnOnce(tokio::net::TcpStream) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = ()> + Send,
    {
        use tokio::io::AsyncReadExt;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (tx, rx) = tokio::sync::oneshot::channel();
        tokio::spawn(async move {
            let (mut sock, _) = listener.accept().await.unwrap();
            let mut raw = Vec::new();
            let mut buf = [0u8; 8192];
            // Headers, then as much body as Content-Length says.
            loop {
                let n = sock.read(&mut buf).await.unwrap_or(0);
                if n == 0 {
                    break;
                }
                raw.extend_from_slice(&buf[..n]);
                let text = String::from_utf8_lossy(&raw).to_string();
                if let Some(end) = text.find("\r\n\r\n") {
                    let len = text[..end]
                        .lines()
                        .find_map(|l| l.to_ascii_lowercase().strip_prefix("content-length:").map(|v| v.trim().parse::<usize>().unwrap_or(0)))
                        .unwrap_or(0);
                    if raw.len() >= end + 4 + len {
                        let _ = tx.send(text[end + 4..].to_string());
                        break;
                    }
                }
            }
            script(sock).await;
        });
        (format!("http://{addr}/v1/messages"), rx)
    }

    #[tokio::test]
    pub(super) async fn the_request_carries_its_body_and_the_stream_comes_back_whole() {
        use tokio::io::AsyncWriteExt;
        // Regression: from Phase 2 until this change, `call` built the body, traced it,
        // and never attached it — every request through the one client went out empty.
        let (url, body) = fake_server(|mut sock| async move {
            let head = "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\nconnection: close\r\n\r\n";
            let _ = sock.write_all(head.as_bytes()).await;
            let _ = sock.write_all(STREAM.as_bytes()).await;
            let _ = sock.shutdown().await;
        })
        .await;
        let req = LlmRequest::new("claude-opus-5", 1024).messages(vec![json!({"role":"user","content":"hi"})]);
        let cancel = Arc::new(AtomicBool::new(false));
        let reply = call_at(&url, &req, "k", &cancel, Some(std::time::Duration::from_secs(5))).await.unwrap().unwrap();
        assert_eq!(reply.tool_uses().len(), 2);
        let sent: Value = serde_json::from_str(&body.await.unwrap()).expect("the request had a JSON body");
        assert_eq!(sent["model"], "claude-opus-5");
        assert_eq!(sent["messages"][0]["content"], "hi");
        assert_eq!(sent["stream"], true);
    }

    #[tokio::test]
    pub(super) async fn a_provider_that_goes_quiet_ends_the_call_within_the_limit() {
        use tokio::io::AsyncWriteExt;
        let (url, _body) = fake_server(|mut sock| async move {
            let head = "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\n\r\n";
            let _ = sock.write_all(head.as_bytes()).await;
            let _ = sock.write_all(STREAM.lines().take(2).map(|l| format!("{l}\n")).collect::<String>().as_bytes()).await;
            let _ = sock.flush().await;
            // …and then nothing, with the connection held open.
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
        })
        .await;
        let req = LlmRequest::new("claude-opus-5", 1024).messages(vec![json!({"role":"user","content":"hi"})]);
        let cancel = Arc::new(AtomicBool::new(false));
        let began = std::time::Instant::now();
        let out = call_at(&url, &req, "k", &cancel, Some(std::time::Duration::from_secs(1))).await;
        assert!(matches!(out, Err(CallError::Stalled(_))), "{out:?}");
        assert!(began.elapsed() < std::time::Duration::from_secs(4), "took {:?}", began.elapsed());
    }

    #[tokio::test]
    pub(super) async fn a_provider_that_never_answers_at_all_is_a_stall_too() {
        // The socket is moved into the future and held: dropping it would close the
        // connection, which is a failure the client can see, not a silence.
        let (url, _body) = fake_server(|sock| async move {
            let _held = sock;
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
        })
        .await;
        let req = LlmRequest::new("claude-opus-5", 1024);
        let cancel = Arc::new(AtomicBool::new(false));
        let out = call_at(&url, &req, "k", &cancel, Some(std::time::Duration::from_secs(1))).await;
        assert!(matches!(out, Err(CallError::Stalled(_))), "{out:?}");
    }
