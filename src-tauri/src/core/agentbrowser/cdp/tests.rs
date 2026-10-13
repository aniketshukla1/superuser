use super::*;

    /// A stand-in for Chrome: answers every command with its own method name, and can
    /// be told to speak first or to hang up.
    async fn fake_chrome() -> (Arc<Cdp>, tokio::sync::mpsc::UnboundedSender<&'static str>) {
        let (ours, theirs) = tokio::io::duplex(1 << 20);
        let (our_read, our_write) = tokio::io::split(ours);
        let (mut their_read, mut their_write) = tokio::io::split(theirs);
        let cdp = Cdp::start(our_read, our_write);
        let (say, mut said) = tokio::sync::mpsc::unbounded_channel::<&'static str>();
        tokio::spawn(async move {
            let mut buf = Vec::new();
            let mut chunk = [0u8; 4096];
            loop {
                tokio::select! {
                    n = their_read.read(&mut chunk) => {
                        let Ok(n) = n else { break };
                        if n == 0 { break; }
                        buf.extend_from_slice(&chunk[..n]);
                        while let Some(end) = buf.iter().position(|b| *b == 0) {
                            let msg: Vec<u8> = buf.drain(..=end).collect();
                            let v: Value = serde_json::from_slice(&msg[..msg.len() - 1]).unwrap();
                            let reply = if v["method"] == "Boom.fail" {
                                json!({"id": v["id"], "error": {"code": -32000, "message": "No node with given id found"}})
                            } else if v["method"] == "Slow.never" {
                                continue;
                            } else {
                                json!({"id": v["id"], "result": {"echo": v["method"], "session": v["sessionId"]}})
                            };
                            let mut bytes = serde_json::to_vec(&reply).unwrap();
                            bytes.push(0);
                            their_write.write_all(&bytes).await.unwrap();
                        }
                    }
                    cmd = said.recv() => {
                        match cmd {
                            Some("event") => {
                                let mut bytes = serde_json::to_vec(&json!({"method": "Page.frameNavigated", "params": {"frame": {"id": "F"}}, "sessionId": "S1"})).unwrap();
                                bytes.push(0);
                                their_write.write_all(&bytes).await.unwrap();
                            }
                            _ => break, // hang up
                        }
                    }
                }
            }
        });
        (cdp, say)
    }

    #[tokio::test]
    async fn answers_find_their_callers_and_errors_come_back_as_words() {
        let (cdp, _say) = fake_chrome().await;
        let r = cdp.call(Some("S1"), "Page.navigate", json!({"url": "https://example.com"}), Duration::from_secs(2)).await.unwrap();
        assert_eq!(r["echo"], "Page.navigate");
        assert_eq!(r["session"], "S1");
        let e = cdp.call(None, "Boom.fail", json!({}), Duration::from_secs(2)).await.unwrap_err();
        assert_eq!(e, "No node with given id found");
    }

    #[tokio::test]
    async fn events_are_broadcast_with_their_session() {
        let (cdp, say) = fake_chrome().await;
        let mut rx = cdp.subscribe();
        say.send("event").unwrap();
        let ev = tokio::time::timeout(Duration::from_secs(2), rx.recv()).await.unwrap().unwrap();
        assert_eq!(ev.method, "Page.frameNavigated");
        assert_eq!(ev.session.as_deref(), Some("S1"));
    }

    #[tokio::test]
    async fn a_timeout_names_the_method_and_never_the_params() {
        let (cdp, _say) = fake_chrome().await;
        let e = cdp.call(None, "Slow.never", json!({"text": "hunter2"}), Duration::from_millis(300)).await.unwrap_err();
        assert!(e.contains("Slow.never") && !e.contains("hunter2"), "{e}");
    }

    #[tokio::test]
    async fn stop_abandons_a_wait_and_a_hang_up_fails_everyone_waiting() {
        let (cdp, say) = fake_chrome().await;
        let cancel = std::sync::Arc::new(AtomicBool::new(false));
        let c = cancel.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(200)).await;
            c.store(true, Ordering::SeqCst);
        });
        let e = cdp.call_with(None, "Slow.never", json!({}), Duration::from_secs(10), Some(&cancel)).await.unwrap_err();
        assert_eq!(e, STOPPED);

        let waiting = {
            let cdp = cdp.clone();
            tokio::spawn(async move { cdp.call(None, "Slow.never", json!({}), Duration::from_secs(10)).await })
        };
        tokio::time::sleep(Duration::from_millis(100)).await;
        say.send("hang up").unwrap();
        let e = tokio::time::timeout(Duration::from_secs(3), waiting).await.unwrap().unwrap().unwrap_err();
        assert_eq!(e, CLOSED);
        assert!(cdp.is_closed());
    }
