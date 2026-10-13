use super::*;

    /// One rendezvous, two shapes of reply.
    #[test]
    pub(super) fn a_reply_reads_the_same_whether_it_was_typed_or_picked() {
        assert_eq!(reply_text(&json!({ "answer": "  Q4  " })), "Q4");
        // The card's reply travels as JSON, because `core::ask` joins a waiting run
        // to a reply and deliberately knows nothing about what was asked.
        let picked = reply_text(&json!({ "answers": ["Presented, now piloting", null] }));
        assert!(picked.contains("Presented, now piloting"), "got: {picked}");
        assert!(picked.starts_with("{\"answers\""), "got: {picked}");
        // Nothing usable is nothing at all — never a stray frame becoming an answer.
        assert_eq!(reply_text(&json!({})), "");
        assert_eq!(reply_text(&json!({ "answers": [] })), "");
        assert_eq!(reply_text(&json!({ "answer": "   " })), "");
    }

    #[test]
    pub(super) fn a_parked_ask_offers_its_labels_as_quick_replies() {
        // The Inbox card speaks in plain strings; the rich card is the live path.
        let args = json!({ "questions": [{ "question": "Which?", "options": [
            { "label": "Q3", "description": "the closed quarter" },
            { "label": "Q4", "description": "the open one" }
        ]}]});
        assert_eq!(inbox_options(&args), vec!["Q3", "Q4"]);
        // …and the one-question shorthand, which is what models mostly send.
        assert_eq!(inbox_options(&json!({ "options": ["a", "b"] })), vec!["a", "b"]);
        assert!(inbox_options(&json!({})).is_empty());
    }

    #[test]
    pub(super) fn an_unknown_kind_is_dropped_rather_than_forwarded_malformed() {
        assert!(gui_frame(&json!({"type": "start", "text": "working…"})).is_none());
        assert!(gui_frame(&json!({})).is_none());
    }

    /// The bug this exists for: 8765 was hard-coded with one attempt, so anything already
    /// holding it — most often the app's own dev build, still running — took the whole app
    /// down. The window opened, the API never started, and the GUI failed every request
    /// with nothing on screen to say why.
    #[tokio::test]
    pub(super) async fn a_taken_port_falls_back_instead_of_failing() {
        // Hold the port the way another instance of the app would.
        let squatter = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let taken = squatter.local_addr().unwrap().port();

        let (listener, fell_back) = bind_loopback(taken)
            .await
            .expect("a taken port must not be fatal — it used to be");
        let got = listener.port();

        assert!(fell_back, "must report that it did not get the port it asked for");
        assert_ne!(got, taken, "cannot have bound the port someone else is holding");
        assert_ne!(got, 0, "an OS-assigned port is a real port");

        // And the ordinary case is unchanged: a free port is taken as-is, not fallen
        // back to.
        //
        // Retried with a fresh port each time, because the obvious version — release
        // the squatter's port and immediately re-bind it — is only *usually* true. A
        // just-freed ephemeral port can be in TIME_WAIT or claimed by another process
        // between the drop and the bind, and then this failed on a property that was
        // never wrong. Rare enough to look like a mystery, common enough to see under
        // repeated runs.
        drop(squatter);
        let mut asked_and_got = false;
        for _ in 0..5 {
            let probe = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let free = probe.local_addr().unwrap().port();
            drop(probe);
            let Ok((l2, fb2)) = bind_loopback(free).await else { continue };
            if l2.port() == free {
                assert!(!fb2, "binding the port you asked for is not a fallback");
                asked_and_got = true;
                break;
            }
        }
        assert!(asked_and_got, "a free port should bind as itself, and five tries all lost the race");
    }

    /// The other side of the same port. A squatter on `[::1]:p` receives what `localhost`
    /// clients send to p, token included, so p is not ours while it holds that side.
    #[tokio::test]
    pub(super) async fn a_port_held_on_the_ipv6_loopback_is_not_ours() {
        let Ok(squatter) = tokio::net::TcpListener::bind("[::1]:0").await else {
            // No IPv6 loopback on this machine: nothing there to squat, nothing to test.
            return;
        };
        let taken = squatter.local_addr().unwrap().port();
        let Ok((listener, fell_back)) = bind_loopback(taken).await else {
            panic!("a port squatted on ::1 must fall back, not fail");
        };
        assert!(fell_back, "127.0.0.1:{taken} was free, but its ::1 side was not — that is not our port");
        assert_ne!(listener.port(), taken);
        assert!(listener.v6.is_some(), "the port it fell back to is held on both sides");
        let v6_port = listener.v6.as_ref().unwrap().local_addr().unwrap().port();
        assert_eq!(v6_port, listener.port(), "one port, both loopbacks");
    }

    /// Both sides answer, and a stop that was asked for reads as one — the supervisor
    /// restarts only what stopped on its own.
    #[tokio::test]
    pub(super) async fn both_loopbacks_serve_and_an_asked_stop_is_not_a_failure() {
        async fn hi() -> &'static str {
            "hi"
        }
        let (listener, _) = bind_loopback(0).await.unwrap();
        let port = listener.port();
        let has_v6 = listener.v6.is_some();
        let (stop, ended) = serve_both(listener, Router::new().route("/hi", get(hi)));
        let http = crate::core::egress::client("test");
        let v4 = http.get(format!("http://127.0.0.1:{port}/hi")).send().await.unwrap().text().await.unwrap();
        assert_eq!(v4, "hi");
        if has_v6 {
            let v6 = http.get(format!("http://[::1]:{port}/hi")).send().await.unwrap().text().await.unwrap();
            assert_eq!(v6, "hi", "the IPv6 side is served, not just held");
        }
        stop.send(true).unwrap();
        let ending = tokio::time::timeout(std::time::Duration::from_secs(5), ended).await.unwrap().unwrap();
        assert_eq!(ending, Ending::Stopped);
    }

    /// Streaks are the figure people check, and the one an off-by-one quietly ruins — so
    /// this drives `compute_stats` with dates chosen for the purpose rather than asserting
    /// invariants against whatever happens to be on disk.
    #[test]
    pub(super) fn stats_counts_days_of_work_not_sessions() {
        use chrono::TimeZone;
        let now = chrono::Local.with_ymd_and_hms(2026, 6, 10, 12, 0, 0).unwrap();
        let day = |y, m, d, h| {
            chrono::Local.with_ymd_and_hms(y, m, d, h, 0, 0).unwrap().timestamp() as f64
        };
        let sess = |at: f64, msgs: usize, model: &str| {
            // Through the real constructor, so a field added to Session cannot leave this
            // fixture silently unlike the thing it stands for.
            let mut s = crate::core::sessions::Session::new("assistant", "/tmp/ws", model);
            s.created_at = at;
            s.updated_at = at;
            for _ in 0..msgs {
                s.messages.push(crate::core::sessions::Message::new("user", "x"));
            }
            s
        };

        // 8th, 9th, 10th = a 3-day run ending today. Two sessions on the 9th are still ONE
        // day. The 1st is an older, unconnected day.
        let owned = vec![
            sess(day(2026, 6, 1, 9), 2, "m1"),
            sess(day(2026, 6, 8, 9), 1, "m1"),
            sess(day(2026, 6, 9, 9), 1, "m2"),
            sess(day(2026, 6, 9, 14), 3, "m1"),
            sess(day(2026, 6, 10, 9), 1, "m1"),
        ];
        let refs: Vec<&crate::core::sessions::Session> = owned.iter().collect();
        let out = compute_stats(&refs, now);

        assert_eq!(out["sessions"], 5);
        assert_eq!(out["messages"], 8);
        assert_eq!(out["active_days"], 4, "two sessions on the 9th are one day of work");
        assert_eq!(out["current_streak"], 3, "8th, 9th and 10th run up to today");
        assert_eq!(out["longest_streak"], 3);
        assert_eq!(out["favorite_model"], "m1");
        assert_eq!(out["peak_hour"], 9);

        // A session with no usable timestamp is left out of the histogram rather than filed
        // under 1970 — the sessions already on disk predate created_at being recorded.
        let mut undated = owned.clone();
        undated.push(sess(0.0, 1, "m1"));
        let refs2: Vec<&crate::core::sessions::Session> = undated.iter().collect();
        let out2 = compute_stats(&refs2, now);
        assert_eq!(out2["sessions"], 6, "it still counts as a session");
        assert_eq!(out2["active_days"], 4, "but not as a day, and certainly not as 1970");
        let days: Vec<&str> = out2["days"].as_array().unwrap().iter().map(|d| d["date"].as_str().unwrap()).collect();
        assert!(!days.iter().any(|d| d.starts_with("1970")), "{days:?}");

        // Yesterday still counts as current: a streak must not read as broken at 00:01
        // before you have had the chance to work.
        let tomorrow = chrono::Local.with_ymd_and_hms(2026, 6, 11, 0, 1, 0).unwrap();
        assert_eq!(compute_stats(&refs, tomorrow)["current_streak"], 3);
        // But two clear days is a broken streak.
        let later = chrono::Local.with_ymd_and_hms(2026, 6, 12, 9, 0, 0).unwrap();
        assert_eq!(compute_stats(&refs, later)["current_streak"], 0);

        // No token figure is offered: nothing in this crate produces one.
        assert!(out.get("total_tokens").is_none());
    }
