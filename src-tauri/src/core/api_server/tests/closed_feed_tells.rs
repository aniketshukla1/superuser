use super::*;

    #[test]
    pub(super) fn bearer_auth_is_exact() {
        let mut h = HeaderMap::new();
        h.insert("authorization", HeaderValue::from_static("Bearer secret123"));
        assert!(authed(&h, "secret123"));
        assert!(!authed(&h, "secret124"));
        assert!(!authed(&h, "secret"));

        // wrong scheme, missing header, and raw token all fail closed
        let mut h2 = HeaderMap::new();
        h2.insert("authorization", HeaderValue::from_static("Basic secret123"));
        assert!(!authed(&h2, "secret123"));
        assert!(!authed(&HeaderMap::new(), "secret123"));
        let mut h3 = HeaderMap::new();
        h3.insert("authorization", HeaderValue::from_static("secret123"));
        assert!(!authed(&h3, "secret123"));
    }

    #[test]
    pub(super) fn the_bearer_scheme_is_case_insensitive_but_the_token_is_not() {
        // RFC 7235 makes the scheme case-insensitive and real OpenAI-compatible
        // clients send `bearer`; the token itself still has to match exactly.
        for scheme in ["Bearer", "bearer", "BEARER"] {
            let mut h = HeaderMap::new();
            h.insert("authorization", HeaderValue::from_str(&format!("{scheme} secret123")).unwrap());
            assert!(authed(&h, "secret123"), "{scheme} should be accepted");
            assert!(!authed(&h, "Secret123"), "{scheme} must not accept a differently-cased token");
        }
    }

    #[test]
    pub(super) fn token_comparison_does_not_short_circuit() {
        // `==` returns at the first differing byte, which tells a caller how much
        // of the token it guessed. Same answers, no early exit.
        assert!(secret_eq("abc", "abc"));
        assert!(!secret_eq("abc", "abd"));
        assert!(!secret_eq("abc", "abcd"), "a prefix is not a match");
        assert!(!secret_eq("", "abc"));
        assert!(secret_eq("", ""));
    }

    #[test]
    pub(super) fn unauthenticated_health_says_nothing_about_the_user() {
        // /v1/health is the one route the guard exempts, and CORS is any-origin
        // on a fixed port — so whatever this returns is readable by any page the
        // user happens to visit. It may say the server is up. It may not name
        // their workspace (an absolute path carrying the account name), their
        // model, or anything else about how the app is set up.
        let body = public_health();
        let obj = body.as_object().expect("an object");
        assert_eq!(obj["status"], "ok", "liveness is the point of the route");
        let allowed = ["ok", "service", "status"];
        let leaked: Vec<&String> = obj.keys().filter(|k| !allowed.contains(&k.as_str())).collect();
        assert!(leaked.is_empty(), "these are readable by any website: {leaked:?}");
    }

    #[test]
    pub(super) fn the_gui_token_header_is_accepted() {
        // The GUI authenticates with X-Superuser-Token, not a bearer — both must work,
        // and neither may pass with the wrong value.
        let mut h = HeaderMap::new();
        h.insert("x-superuser-token", HeaderValue::from_static("secret123"));
        assert!(authed(&h, "secret123"));
        assert!(!authed(&h, "nope"));
    }

    /// A step's status is only red when the tool really said so — anything else paints a
    /// failed step under a perfectly good answer.
    #[test]
    pub(super) fn tool_status_flags_failures_and_nothing_else() {
        use crate::commands::tool_status;
        assert_eq!(tool_status("error: describe the subtask"), "error");
        assert_eq!(tool_status("  Error: no such file"), "error");
        assert_eq!(tool_status("unknown skill: pdf"), "error");
        assert_eq!(tool_status("subagent failed: timed out"), "error");

        assert_eq!(tool_status("wrote latency-check.txt"), "ok");
        assert_eq!(tool_status(""), "ok");
        // Prose that merely mentions trouble is a successful read, not a failed step.
        assert_eq!(tool_status("The log explains what an error is and how to fix it."), "ok");
    }

    /// A socket following a turn learns when that turn ends.
    ///
    /// The regression: the handler kept the `RunFeed` it got from `feed_open` alive for
    /// the whole forwarding loop. That tuple carries a broadcast Sender, so "all senders
    /// dropped" — the only signal the loop had that the turn was over — could never fire.
    /// The turn finished and persisted, the socket stayed parked on recv(), and the
    /// client sat on its last tool step until you switched chats and back.
    #[tokio::test]
    pub(super) async fn a_closed_feed_tells_its_subscribers_the_turn_ended() {
        let id = "feed-close-test";
        let feed = feed_open(id, std::env::temp_dir().join(format!("su-feedtest-{id}")));
        let mut sub = feed.1.subscribe();

        // The forwarder task's clone; the socket drops its own (the fix).
        let forwarder = feed.clone();

        feed_push(&forwarder, json!({ "type": "tool_proposed" }));
        assert_eq!(sub.recv().await.unwrap()["type"], "tool_proposed");

        // Turn ends: the run closes the feed, then the forwarder finishes.
        feed_close(id);
        drop(forwarder);

        // The socket's own handle is still alive here — the old bug. Nothing has told
        // this subscriber anything, even though the turn is over.
        assert!(
            matches!(sub.try_recv(), Err(tokio::sync::broadcast::error::TryRecvError::Empty)),
            "a held Sender masks the turn's end — this is what hung the socket",
        );
        drop(feed);

        assert!(
            matches!(sub.recv().await, Err(tokio::sync::broadcast::error::RecvError::Closed)),
            "a subscriber must see the feed close, or the socket never sends turn_done",
        );
    }

    /// Frames buffered before the close still arrive: ending the turn must not truncate
    /// the steps the client hasn't read yet.
    #[tokio::test]
    pub(super) async fn frames_queued_before_the_close_are_still_delivered() {
        let id = "feed-drain-test";
        let feed = feed_open(id, std::env::temp_dir().join(format!("su-feedtest-{id}")));
        let mut sub = feed.1.subscribe();
        let forwarder = feed.clone();
        drop(feed);

        feed_push(&forwarder, json!({ "type": "assistant_delta" }));
        feed_close(id);
        drop(forwarder);

        assert_eq!(sub.recv().await.unwrap()["type"], "assistant_delta");
        assert!(sub.recv().await.is_err());
    }

    #[test]
    pub(super) fn the_stats_report_what_the_cache_did_and_what_it_cost() {
        // HARNESS-PLAN Phase 0: "a turn on Opus 5 prints its own cost". These are the
        // aggregate numbers every later phase is measured against, so they have to
        // carry the cache split — a rollup reporting only input+output cannot show
        // Phase 3 working or failing.
        use crate::core::sessions::{Message, Session, TokenUsage};
        let now = chrono::Local::now();
        let mut s = Session::new("assistant", "/tmp/ws", "claude-opus-5");
        s.created_at = now.timestamp() as f64;
        s.updated_at = s.created_at;
        // One cold step then one warm one: 15k written, then 15k read back.
        s.messages.push(Message::new("assistant", "a").with_usage(Some(TokenUsage { input: 0, output: 100, cache_read: 0, cache_write: 15_000, ..Default::default() })));
        s.messages.push(Message::new("assistant", "b").with_usage(Some(TokenUsage { input: 0, output: 100, cache_read: 15_000, cache_write: 0, ..Default::default() })));
        let owned = vec![s];
        let refs: Vec<&Session> = owned.iter().collect();
        let out = compute_stats(&refs, now);

        let m = &out["models"][0];
        assert_eq!(m["cache_write"], 15_000);
        assert_eq!(m["cache_read"], 15_000);
        assert_eq!(m["tokens"], 30_200, "cache tokens are tokens the model saw");
        assert_eq!(m["cache_hit_rate"], 0.5);
        // 15k write @ $6.25/M + 15k read @ $0.50/M + 200 out @ $25/M
        let cost = m["cost_usd"].as_f64().unwrap();
        assert!((cost - (0.09375 + 0.0075 + 0.005)).abs() < 1e-6, "cost: {cost}");
        assert!((out["cost_usd"].as_f64().unwrap() - cost).abs() < 1e-9);
    }
