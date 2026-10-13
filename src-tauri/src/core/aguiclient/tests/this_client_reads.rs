use super::*;

    #[tokio::test]
    pub(super) async fn a_redirect_the_floor_refuses_is_not_followed() {
        let (port, seen) = serve(|_| vec![("/start", redirect("http://169.254.169.254/latest/meta-data".into()))]).await;
        let http = crate::core::egress::client_no_follow("test");
        let refused = run_under(
            &http,
            &format!("http://127.0.0.1:{port}/start"),
            Some("secret-token"),
            &input_for("t", "r", &[], "hi"),
            floor_allowing_the_test_server,
            &mut |_| {},
        )
        .await
        .unwrap_err();
        assert!(refused.starts_with("Refused. That agent redirected somewhere"), "{refused}");
        assert_eq!(seen.lock().unwrap().len(), 1, "nothing went to the address it was bounced to");
    }

    #[tokio::test]
    pub(super) async fn an_address_that_isnt_an_event_stream_says_so() {
        let json = "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: 2\r\nconnection: close\r\n\r\n{}".to_string();
        let (port, _) = serve(move |_| vec![("/agent", json)]).await;
        let http = crate::core::egress::client_no_follow("test");
        let e = run_under(&http, &format!("http://127.0.0.1:{port}/agent"), None, &input_for("t", "r", &[], "hi"), floor_allowing_the_test_server, &mut |_| {})
            .await
            .unwrap_err();
        assert!(e.contains("not with an event stream (application/json)"), "{e}");
        // …and the real floor never lets a run near this machine at all.
        let e = run(&http, &format!("http://127.0.0.1:{port}/agent"), None, &input_for("t", "r", &[], "hi"), &mut |_| {}).await.unwrap_err();
        assert!(e.starts_with("Refused."), "{e}");
    }

    #[test]
    pub(super) fn a_connection_test_needs_a_finished_run_with_words_in_it() {
        let ok = Answer { text: "Hello!".into(), steps: vec![], error: None };
        assert_eq!(judge_test(&ok).unwrap(), "It answered: “Hello!”");
        assert!(judge_test(&Answer::default()).unwrap_err().contains("carried no reply"));
        let failed = Answer { text: "Hel".into(), steps: vec![], error: Some("no model".into()) };
        assert!(judge_test(&failed).unwrap_err().contains("no model"));
    }

    #[test]
    pub(super) fn this_client_reads_what_this_apps_own_agui_server_sends() {
        // The two halves of Phase 7 speak one dialect or neither is worth much: a
        // Superuser assistant served over AG-UI has to be readable by a Superuser
        // assistant pointed at it.
        use crate::core::agui;
        let ids = agui::Ids::from_input(&json!({ "threadId": "t1", "runId": "r1" }));
        let mut open = false;
        let mut body = agui::sse(&agui::run_started(&ids));
        for ours in [json!({ "type": "text", "text": "Forty " }), json!({ "type": "text", "text": "two." })] {
            for event in agui::events_for(&ours, &ids, &mut open) {
                body.push_str(&agui::sse(&event));
            }
        }
        if let Some(end) = agui::text_end(&ids, open) {
            body.push_str(&agui::sse(&end));
        }
        body.push_str(&agui::sse(&agui::run_finished(&ids)));
        let answer = read_stream(&body);
        assert_eq!(answer.text, "Forty two.");
        assert_eq!(answer.error, None);

        let failed = format!("{}{}", agui::sse(&agui::run_started(&ids)), agui::sse(&agui::run_error(&ids, "no model")));
        assert_eq!(read_stream(&failed).error.as_deref(), Some("no model"));
    }

    #[test]
    pub(super) fn where_a_remote_agent_may_live_is_the_same_floor_a_connected_app_meets() {
        assert_eq!(endpoint_problem("https://agent.example.com/run"), None);
        assert!(endpoint_problem("http://agent.example.com/run").is_some());
        assert!(endpoint_problem("https://localhost:4200/run").is_some());
        assert!(endpoint_problem("https://agent.example.com/run?token=abc").is_some());
    }
