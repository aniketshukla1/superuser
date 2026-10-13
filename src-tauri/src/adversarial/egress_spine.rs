use crate::core::egress;

    #[test]
    fn a_public_host_may_not_redirect_onto_this_machine() {
        // The boundary rule, stated directly. Crossing in is refused; staying on
        // either side is fine, or the local API and OAuth's loopback callback break.
        assert!(!egress::redirect_allowed("evil.example", "127.0.0.1"), "public -> loopback");
        assert!(!egress::redirect_allowed("evil.example", "169.254.169.254"), "public -> metadata");
        assert!(!egress::redirect_allowed("evil.example", "[::ffff:127.0.0.1]"), "public -> mapped loopback");
        assert!(!egress::redirect_allowed("evil.example", "192.168.1.5"), "public -> LAN");
        assert!(egress::redirect_allowed("127.0.0.1", "127.0.0.1"), "loopback -> loopback must still work");
        assert!(egress::redirect_allowed("localhost", "127.0.0.1"), "the local API redirects to itself");
        assert!(egress::redirect_allowed("a.example", "b.example"), "public -> public");
    }

    #[tokio::test]
    async fn a_redirect_that_stays_on_this_machine_is_still_followed() {
        // web_fetch / skill install validate the URL the *user* supplied and then
        // hand it to reqwest, whose default policy follows up to 10 redirects.
        // Nothing re-validates the hop, so a public URL can bounce the agent onto
        // loopback or link-local. This probe proves redirects are followed.
        let hit = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

        // Target: records that it was reached.
        let target = {
            let hit = hit.clone();
            let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let port = l.local_addr().unwrap().port();
            tokio::spawn(async move {
                if let Ok((mut s, _)) = l.accept().await {
                    use tokio::io::{AsyncReadExt, AsyncWriteExt};
                    let mut b = [0u8; 1024];
                    let _ = s.read(&mut b).await;
                    hit.store(true, std::sync::atomic::Ordering::SeqCst);
                    let _ = s.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 6\r\n\r\nSECRET").await;
                }
            });
            port
        };

        // Redirector: 302s to the target.
        let redirector = {
            let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let port = l.local_addr().unwrap().port();
            tokio::spawn(async move {
                if let Ok((mut s, _)) = l.accept().await {
                    use tokio::io::{AsyncReadExt, AsyncWriteExt};
                    let mut b = [0u8; 1024];
                    let _ = s.read(&mut b).await;
                    let body = format!(
                        "HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:{target}/secret\r\nContent-Length: 0\r\n\r\n"
                    );
                    let _ = s.write_all(body.as_bytes()).await;
                }
            });
            port
        };

        let resp = egress::client("web")
            .get(format!("http://127.0.0.1:{redirector}/start"))
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await;
        let followed = hit.load(std::sync::atomic::Ordering::SeqCst);
        let body = match resp {
            Ok(r) => r.text().await.unwrap_or_default(),
            Err(e) => format!("<err {e}>"),
        };
        assert!(
            followed && body == "SECRET",
            "a loopback -> loopback redirect must still be followed, or the local API and \
             OAuth's callback break (followed={followed}, body={body:?})"
        );
    }

    #[test]
    fn the_log_never_loses_a_line_to_rotation() {
        // Rotation is read-modify-write under a lock; a failed temp write silently
        // leaves the log unbounded, and the keep-half slice drops a partial line.
        // Shares the process-wide log location with egress's own tests.
        let _lock = egress::test_lock();
        let d = super::scratch("egress-rot");
        egress::set_data_dir(&d);
        for i in 0..4_000 {
            egress::record_for_test("example.com", &format!("p{i}"), 1);
        }
        let text = std::fs::read_to_string(d.join("egress.jsonl")).unwrap_or_default();
        let bad: Vec<&str> = text.lines().filter(|l| !l.is_empty() && serde_json::from_str::<serde_json::Value>(l).is_err()).collect();
        assert!(bad.is_empty(), "{} unparseable line(s) survived rotation: {:?}", bad.len(), bad.first());
        std::fs::remove_dir_all(&d).ok();
    }
