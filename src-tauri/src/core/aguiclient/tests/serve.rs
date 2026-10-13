use super::*;

    pub(super) fn stream(lines: &[&str]) -> String {
        lines.iter().map(|l| format!("data: {l}\n\n")).collect()
    }

    #[test]
    pub(super) fn a_stream_is_read_into_its_answer_and_its_steps() {
        let body = stream(&[
            r#"{"type":"RUN_STARTED","threadId":"t","runId":"r"}"#,
            r#"{"type":"TEXT_MESSAGE_START","messageId":"m","role":"assistant"}"#,
            r#"{"type":"TEXT_MESSAGE_CONTENT","messageId":"m","delta":"About "}"#,
            r#"{"type":"TOOL_CALL_START","toolCallId":"c1","toolCallName":"search_issues"}"#,
            r#"{"type":"TOOL_CALL_END","toolCallId":"c1"}"#,
            r#"{"type":"TEXT_MESSAGE_CONTENT","messageId":"m","delta":"nine months."}"#,
            r#"{"type":"TEXT_MESSAGE_END","messageId":"m"}"#,
            r#"{"type":"RUN_FINISHED","threadId":"t","runId":"r"}"#,
        ]);
        let answer = read_stream(&body);
        assert_eq!(answer.text, "About nine months.");
        assert_eq!(answer.steps, vec!["search_issues"]);
        assert_eq!(answer.error, None);
    }

    #[test]
    pub(super) fn a_run_that_never_finished_is_not_an_answer() {
        let cut = stream(&[
            r#"{"type":"RUN_STARTED","threadId":"t","runId":"r"}"#,
            r#"{"type":"TEXT_MESSAGE_CONTENT","messageId":"m","delta":"About nine"}"#,
        ]);
        let answer = read_stream(&cut);
        assert_eq!(answer.text, "About nine");
        assert!(answer.error.unwrap().contains("ended before that run finished"), "half an answer is not an answer");

        let failed = stream(&[
            r#"{"type":"RUN_STARTED","threadId":"t","runId":"r"}"#,
            r#"{"type":"RUN_ERROR","message":"no model configured"}"#,
        ]);
        assert_eq!(read_stream(&failed).error.as_deref(), Some("no model configured"));
        // Junk between events is skipped rather than fatal: keep-alives and comments are
        // ordinary in an SSE stream.
        let noisy = format!(": keep-alive\n\n{}", stream(&[r#"{"type":"RUN_FINISHED"}"#]));
        assert_eq!(read_stream(&noisy).error, None);
    }

    #[test]
    pub(super) fn the_request_offers_the_conversation_and_no_tools() {
        let input = input_for("t1", "r1", &[("user".into(), "hi".into()), ("assistant".into(), "hello".into())], "and now?");
        assert_eq!(input["threadId"], "t1");
        let messages = input["messages"].as_array().unwrap();
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[2]["role"], "user");
        assert_eq!(messages[2]["content"], "and now?");
        assert_eq!(input["tools"].as_array().unwrap().len(), 0, "a remote agent's tools are its own");
    }

    /// A tiny HTTP server that answers each path with a canned response and writes down the
    /// request heads it saw, so a test can check what was sent where.
    pub(super) async fn serve(routes: impl FnOnce(u16) -> Vec<(&'static str, String)>) -> (u16, std::sync::Arc<std::sync::Mutex<Vec<String>>>) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let routes = routes(port);
        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let log = seen.clone();
        tokio::spawn(async move {
            loop {
                let Ok((mut sock, _)) = listener.accept().await else { return };
                let mut raw = Vec::new();
                let mut buf = [0u8; 4096];
                loop {
                    let n = sock.read(&mut buf).await.unwrap_or(0);
                    if n == 0 {
                        break;
                    }
                    raw.extend_from_slice(&buf[..n]);
                    let text = String::from_utf8_lossy(&raw).to_string();
                    if let Some(head_end) = text.find("\r\n\r\n") {
                        let length = text[..head_end]
                            .lines()
                            .find_map(|l| l.to_lowercase().strip_prefix("content-length:").map(|v| v.trim().parse::<usize>().unwrap_or(0)))
                            .unwrap_or(0);
                        if raw.len() >= head_end + 4 + length {
                            break;
                        }
                    }
                }
                let head = String::from_utf8_lossy(&raw).split("\r\n\r\n").next().unwrap_or("").to_string();
                let path = head.split_whitespace().nth(1).unwrap_or("").to_string();
                log.lock().unwrap().push(head);
                let reply = routes
                    .iter()
                    .find(|(p, _)| *p == path)
                    .map(|(_, r)| r.clone())
                    .unwrap_or_else(|| "HTTP/1.1 404 Not Found\r\ncontent-length: 0\r\nconnection: close\r\n\r\n".into());
                let _ = sock.write_all(reply.as_bytes()).await;
                let _ = sock.shutdown().await;
            }
        });
        (port, seen)
    }

    pub(super) fn sse_reply(events: &[&str]) -> String {
        let body = stream(events);
        format!("HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}", body.len())
    }

    /// The production floor with one exception: this machine, where the test server is.
    /// Everything else a hop could reach still meets the real checks.
    pub(super) fn floor_allowing_the_test_server(url: &str) -> Option<String> {
        if url.starts_with("http://127.0.0.1:") || url.starts_with("http://localhost:") {
            None
        } else {
            endpoint_problem(url)
        }
    }

    pub(super) const DONE: &[&str] = &[
        r#"{"type":"RUN_STARTED","threadId":"t","runId":"r"}"#,
        r#"{"type":"TEXT_MESSAGE_CONTENT","messageId":"m","delta":"Hello there."}"#,
        r#"{"type":"RUN_FINISHED","threadId":"t","runId":"r"}"#,
    ];

    #[tokio::test]
    pub(super) async fn a_run_streams_each_event_as_it_arrives() {
        let (port, _) = serve(|_| vec![("/agent", sse_reply(DONE))]).await;
        let http = crate::core::egress::client_no_follow("test");
        let mut kinds = Vec::new();
        let answer = run_under(
            &http,
            &format!("http://127.0.0.1:{port}/agent"),
            None,
            &input_for("t", "r", &[], "hi"),
            floor_allowing_the_test_server,
            &mut |e| kinds.push(e["type"].as_str().unwrap_or("").to_string()),
        )
        .await
        .unwrap();
        assert_eq!(answer.text, "Hello there.");
        assert_eq!(kinds, vec!["RUN_STARTED", "TEXT_MESSAGE_CONTENT", "RUN_FINISHED"]);
    }

    pub(super) fn redirect(to: String) -> String {
        format!("HTTP/1.1 307 Temporary Redirect\r\nlocation: {to}\r\ncontent-length: 0\r\nconnection: close\r\n\r\n")
    }

    pub(super) fn authorization_of(head: &str) -> Option<String> {
        head.lines().find_map(|l| {
            let (name, value) = l.split_once(':')?;
            name.eq_ignore_ascii_case("authorization").then(|| value.trim().to_string())
        })
    }

    #[tokio::test]
    pub(super) async fn a_redirect_to_another_host_goes_without_the_token() {
        // 127.0.0.1 and localhost are one server and two hosts, which is what this needs.
        let (port, seen) = serve(|port| {
            vec![("/start", redirect(format!("http://localhost:{port}/agent"))), ("/agent", sse_reply(DONE))]
        })
        .await;
        let http = crate::core::egress::client_no_follow("test");
        let answer = run_under(
            &http,
            &format!("http://127.0.0.1:{port}/start"),
            Some("secret-token"),
            &input_for("t", "r", &[], "hi"),
            floor_allowing_the_test_server,
            &mut |_| {},
        )
        .await
        .unwrap();
        assert_eq!(answer.text, "Hello there.", "the redirect was followed to the answer");
        let heads = seen.lock().unwrap().clone();
        assert_eq!(heads.len(), 2);
        assert_eq!(authorization_of(&heads[0]).as_deref(), Some("Bearer secret-token"));
        assert_eq!(authorization_of(&heads[1]), None, "a different host is a different party");
        assert!(heads[1].starts_with("POST /agent"), "307 keeps the run a POST: {}", heads[1]);
    }
