use super::*;

    pub(in crate::core::egress::tests) fn scratch() -> PathBuf {
        static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let seq = N.fetch_add(1, Ordering::SeqCst);
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let d = std::env::temp_dir().join(format!("bba-egress-{}-{}-{}", std::process::id(), nanos, seq));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// The test that makes this module a chokepoint instead of a suggestion.
    ///
    /// A helper everyone is supposed to remember is not an enforcement mechanism.
    /// If this fails, someone added a call site that reaches the network without
    /// being recorded, and the Privacy page is now quietly lying — which is worse
    /// than not having one, because it is read as complete.
    #[test]
    pub(in crate::core::egress::tests) fn every_outbound_request_goes_through_here() {
        let mut offenders: Vec<String> = Vec::new();
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let egress = root.join("core").join("egress");
        let mut stack = vec![root];
        while let Some(dir) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&dir) else { continue };
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    stack.push(p);
                    continue;
                }
                if p.extension().and_then(|e| e.to_str()) != Some("rs") {
                    continue;
                }
                // This module is where the one legitimate construction lives: egress.rs
                // and, since it was split, the egress/ folder beside it.
                if p == egress.with_extension("rs") || p.starts_with(&egress) {
                    continue;
                }
                let Ok(text) = std::fs::read_to_string(&p) else { continue };
                // Every way to reach the network without going through the client
                // above. `reqwest::get` and friends are free functions with a
                // client hidden inside them, which is exactly the shape that would
                // slip past a check that only looked for `Client::new`.
                const DIRECT: &[&str] = &[
                    "reqwest::Client::new()",
                    "reqwest::Client::builder()",
                    "reqwest::Client::default()",
                    "reqwest::get(",
                    "reqwest::blocking",
                ];
                for (i, line) in text.lines().enumerate() {
                    if DIRECT.iter().any(|d| line.contains(d)) {
                        offenders.push(format!("{}:{}", p.display(), i + 1));
                    }
                }
                // Protocols with their own sockets can't be intercepted, so the
                // rule for them is that the file has to declare itself.
                let speaks = ["SmtpTransport::", "imap::ClientBuilder", "TcpStream::connect"];
                if speaks.iter().any(|s| text.contains(s)) && !text.contains("egress::allow") && !module_declares(&p) {
                    offenders.push(format!("{} (opens a socket without egress::allow)", p.display()));
                }
            }
        }
        assert!(
            offenders.is_empty(),
            "these reach the network without going through the spine, so their traffic is invisible \
             to the Privacy page and unaffected by local-only mode. Use `egress::client(purpose)`, \
             or `egress::allow` for protocols that aren't HTTP:\n  {}",
            offenders.join("\n  ")
        );
    }

    #[test]
    pub(in crate::core::egress::tests) fn a_url_becomes_a_host_and_a_path_and_loses_its_query() {
        // The query string is where tokens and search terms live. A privacy log
        // that copies them has become a second place your secrets are kept.
        let (h, p) = split_url("https://api.github.com/user/repos?access_token=ghp_secret").unwrap();
        assert_eq!(h, "api.github.com");
        assert_eq!(p, "/user/repos");

        assert_eq!(split_url("https://EXAMPLE.com").unwrap(), ("example.com".into(), "/".into()));
        assert_eq!(split_url("http://user:pw@host.io:8443/a/b#frag").unwrap().0, "host.io");
        assert_eq!(split_url("http://[::1]:11434/api/chat").unwrap().0, "::1");
        assert!(split_url("not a url").is_none());
    }

    #[test]
    pub(in crate::core::egress::tests) fn what_never_left_the_machine_is_not_egress() {
        for h in ["127.0.0.1", "localhost", "::1", "192.168.1.9", "foo.local"] {
            assert!(is_local(h), "{h} is on this machine");
        }
        for h in ["api.github.com", "api.anthropic.com", "8.8.8.8"] {
            assert!(!is_local(h), "{h} is not");
        }
    }

    #[test]
    pub(in crate::core::egress::tests) fn the_log_rolls_up_by_host() {
        let _lock = exclusive();
        let d = scratch();
        set_data_dir(&d);
        for (host, purpose, sent) in [
            ("api.github.com", "github", 100u64),
            ("api.github.com", "github", 50),
            ("api.anthropic.com", "model", 900),
            ("api.github.com", "web", 10),
        ] {
            record(Egress {
                at: now(),
                purpose: purpose.into(),
                method: "GET".into(),
                host: host.into(),
                path: "/x".into(),
                sent,
                status: Some(200),
                error: None,
                blocked: false,
            });
        }
        let hosts = by_host(&d);
        assert_eq!(hosts[0].host, "api.github.com", "busiest first");
        assert_eq!(hosts[0].requests, 3);
        assert_eq!(hosts[0].sent, 160);
        // Two different features talked to it, and both should be visible.
        assert_eq!(hosts[0].purposes, vec!["github", "web"]);
        assert_eq!(hosts[1].host, "api.anthropic.com");
        assert_eq!(recent(&d, 10).len(), 4);
        assert_eq!(recent(&d, 10)[0].host, "api.github.com", "newest first");
                std::fs::remove_dir_all(&d).ok();
    }

    #[tokio::test]
    pub(in crate::core::egress::tests) async fn local_only_refuses_and_says_so_in_the_log() {
        let _lock = exclusive();
        // The claim `model_router` has always made in its own doc comment — "a hard
        // egress switch" — tested at the one place that can make it true.
        let d = scratch();
        set_data_dir(&d);
        set_local_only(true);

        let err = client("github").get("https://api.github.com/user").send().await.unwrap_err();
        assert!(matches!(err, EgressError::LocalOnly(_)));
        assert!(err.to_string().contains("api.github.com"));

        let log = recent(&d, 10);
        assert_eq!(log.len(), 1, "a refusal is an event worth recording");
        assert!(log[0].blocked);
        assert_eq!(log[0].host, "api.github.com");
        assert_eq!(log[0].status, None, "nothing was sent, so there is no status");
        assert_eq!(by_host(&d)[0].blocked, 1);

        set_local_only(false);
                std::fs::remove_dir_all(&d).ok();
    }

    #[tokio::test]
    pub(in crate::core::egress::tests) async fn local_only_does_not_cut_the_machine_off_from_itself() {
        let _lock = exclusive();
        // Local-only means nothing leaves, not that nothing works: Ollama, the
        // local API and the vault are all on this machine. If this ever fails, the
        // switch has become an off switch for the whole product.
        let d = scratch();
        set_data_dir(&d);
        set_local_only(true);

        // Nothing is listening, so this fails at the transport — the point is
        // *which* error comes back: a connection error means we tried.
        let err = client("model")
            .get("http://127.0.0.1:1/api/tags")
            .timeout(std::time::Duration::from_millis(300))
            .send()
            .await
            .unwrap_err();
        assert!(matches!(err, EgressError::Http(_)), "a local call must not be refused: {err}");
        assert!(recent(&d, 10).is_empty(), "and it is not egress, so it is not in the log");

        set_local_only(false);
                std::fs::remove_dir_all(&d).ok();
    }
