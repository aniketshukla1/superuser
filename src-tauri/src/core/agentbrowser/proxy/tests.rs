use super::*;

    fn head(text: &str) -> Head {
        parse_head(text.as_bytes()).expect("parses")
    }

    #[test]
    fn heads_and_targets_parse_both_ways_chrome_sends_them() {
        let h = head("CONNECT example.com:443 HTTP/1.1\r\nHost: example.com:443\r\n\r\n");
        assert_eq!(target_of(&h).unwrap(), Target { host: "example.com".into(), port: 443, tunnel: true, path: String::new() });

        let h = head("GET http://Example.COM./a/b?q=1 HTTP/1.1\r\nHost: example.com\r\nProxy-Connection: keep-alive\r\n\r\n");
        let t = target_of(&h).unwrap();
        assert_eq!((t.host.as_str(), t.port, t.tunnel, t.path.as_str()), ("example.com", 80, false, "/a/b?q=1"));

        let h = head("CONNECT [::1]:8443 HTTP/1.1\r\n\r\n");
        assert_eq!(target_of(&h).unwrap().host, "::1");
        assert!(target_of(&head("GET /relative HTTP/1.1\r\n\r\n")).is_err());
        assert!(target_of(&head("GET ftp://x.com/ HTTP/1.1\r\n\r\n")).is_err());
        assert!(parse_head(b"GET http://x.com/ HTTP/1.1\r\n").is_none(), "no end of head yet");
        assert!(parse_head(b"garbage\r\n\r\n").is_none());
    }

    #[test]
    fn the_name_floor_refuses_metadata_private_and_unallowed_loopback() {
        assert!(decide_name("169.254.169.254", true).is_err());
        assert!(decide_name("metadata.google.internal", true).is_err());
        assert!(decide_name("metadata.goog", true).is_err());
        assert!(decide_name("10.0.0.8", true).is_err());
        assert!(decide_name("192.168.1.1", true).is_err());
        assert!(decide_name("localhost", false).unwrap_err().contains("Settings ▸ Browser"));
        assert_eq!(decide_name("localhost", true), Ok(true));
        assert_eq!(decide_name("127.0.0.1", true), Ok(true));
        assert_eq!(decide_name("app.localhost", true), Ok(true));
        assert_eq!(decide_name("example.com", false), Ok(false));
    }

    #[test]
    fn the_named_staging_site_may_be_private_and_nothing_else_may() {
        // The exception: Bug Reproduction's staging, named by the person, on a company network.
        assert_eq!(decide_name_or_staging("staging.corp.internal", false, true), Ok(false));
        assert_eq!(decide_name_or_staging("10.0.0.8", false, true), Ok(false));
        assert_eq!(decide_addr_or_staging("10.0.0.8".parse().unwrap(), false, true), Ok(()));
        assert_eq!(decide_addr_or_staging("fd12:3456::8".parse().unwrap(), false, true), Ok(()));
        assert_eq!(decide_addr_or_staging("100.72.0.9".parse().unwrap(), false, true), Ok(()), "a tailnet is a private network");
        // Never, whatever is named: metadata, link-local, reserved.
        for name in ["metadata.google.internal", "metadata.goog", "169.254.169.254", "0.0.0.0", "[fe80::1]"] {
            assert!(decide_name_or_staging(name, true, true).is_err(), "{name}");
        }
        for ip in ["169.254.169.254", "::ffff:169.254.169.254", "0.0.0.0", "224.0.0.1"] {
            assert!(decide_addr_or_staging(ip.parse().unwrap(), false, true).is_err(), "{ip}");
        }
        // Loopback keeps its own switch.
        assert!(decide_name_or_staging("localhost", false, true).unwrap_err().contains("Settings ▸ Browser"));
        assert!(decide_addr_or_staging("127.0.0.1".parse().unwrap(), false, true).is_err());
        // Anything not the staging site: the floor, unchanged.
        assert!(decide_name_or_staging("10.0.0.8", true, false).is_err());
        assert!(decide_addr_or_staging("10.0.0.8".parse().unwrap(), false, false).is_err());
    }

    #[test]
    fn a_public_name_that_resolves_somewhere_private_is_refused() {
        assert!(decide_addr("93.184.216.34".parse().unwrap(), false).is_ok());
        assert!(decide_addr("127.0.0.1".parse().unwrap(), false).unwrap_err().contains("this machine"));
        assert!(decide_addr("127.0.0.1".parse().unwrap(), true).is_ok());
        assert!(decide_addr("169.254.169.254".parse().unwrap(), false).is_err());
        assert!(decide_addr("::ffff:10.0.0.1".parse().unwrap(), false).is_err());
    }

    #[test]
    fn a_plain_http_request_goes_upstream_in_origin_form_without_proxy_headers() {
        let h = head("GET http://example.com/x?y=1 HTTP/1.1\r\nHost: example.com\r\nProxy-Authorization: Basic abc\r\nConnection: keep-alive\r\nAccept: */*\r\n\r\n");
        let out = String::from_utf8(rewrite_for_origin(&h, &target_of(&h).unwrap())).unwrap();
        assert!(out.starts_with("GET /x?y=1 HTTP/1.1\r\n"), "{out}");
        assert!(out.contains("Host: example.com\r\n") && out.contains("Accept: */*\r\n"), "{out}");
        assert!(!out.to_lowercase().contains("proxy-authorization"), "{out}");
        assert!(out.ends_with("Connection: close\r\n\r\n"), "{out}");
    }

    /// The whole path through a real socket: a loopback origin, reached through the proxy
    /// when loopback is allowed, and refused with a sentence when it is not.
    #[tokio::test]
    async fn requests_go_through_when_allowed_and_are_refused_in_words_when_not() {
        let origin = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let origin_port = origin.local_addr().unwrap().port();
        tokio::spawn(async move {
            while let Ok((mut s, _)) = origin.accept().await {
                tokio::spawn(async move {
                    let mut buf = vec![0u8; 4096];
                    let n = s.read(&mut buf).await.unwrap_or(0);
                    let req = String::from_utf8_lossy(&buf[..n]).to_string();
                    let first = req.lines().next().unwrap_or("").to_string();
                    let body = format!("origin saw: {first}");
                    let _ = s
                        .write_all(format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{body}", body.len()).as_bytes())
                        .await;
                });
            }
        });
        let dir = std::env::temp_dir().join(format!("su-proxy-{}-{}", std::process::id(), origin_port));
        std::fs::create_dir_all(&dir).unwrap();
        let allow = Arc::new(AtomicBool::new(true));
        let proxy = start("assistant", dir.clone(), allow.clone()).await.unwrap();

        let ask = |req: String| async move {
            let mut s = TcpStream::connect(("127.0.0.1", proxy.port)).await.unwrap();
            s.write_all(req.as_bytes()).await.unwrap();
            let mut out = String::new();
            let _ = tokio::time::timeout(Duration::from_secs(5), s.read_to_string(&mut out)).await;
            out
        };
        let got = ask(format!("GET http://127.0.0.1:{origin_port}/hello?x=1 HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")).await;
        assert!(got.contains("origin saw: GET /hello?x=1 HTTP/1.1"), "{got}");

        allow.store(false, Ordering::SeqCst);
        let got = ask(format!("GET http://127.0.0.1:{origin_port}/hello HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")).await;
        assert!(got.starts_with("HTTP/1.1 403"), "{got}");
        assert!(got.contains("Settings ▸ Browser"), "{got}");
        let got = ask("CONNECT 169.254.169.254:80 HTTP/1.1\r\n\r\n".into()).await;
        assert!(got.starts_with("HTTP/1.1 403") && got.contains("metadata"), "{got}");

        let refused: Vec<String> = proxy.refusals().into_iter().map(|r| r.host).collect();
        assert_eq!(refused, vec!["127.0.0.1".to_string(), "169.254.169.254".to_string()]);
        let rows = crate::core::audit::query(&dir, &crate::core::audit::Filter { event: Some("browser.blocked".into()), ..Default::default() });
        assert_eq!(rows.len(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }
