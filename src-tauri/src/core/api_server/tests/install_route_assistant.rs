use super::*;

    /// Product Performance's route takes the signals and nothing else, refuses a line that isn't
    /// one, and adds the weekly report once, for this assistant.
    #[test]
    pub(super) fn the_perf_route_takes_only_well_formed_signals_and_adds_the_report_once() {
        let good = "Checkout p95: histogram_quantile(0.95, sum by (le) (rate(http_seconds_bucket[5m])))\n# a comment\nErrors: sum(rate(http_requests_total{code=~\"5..\"}[5m]))";
        assert_eq!(perf_patch(&json!({ "perf_signals": good, "repro_staging": "x" })), Ok(json!({ "perf_signals": good })));
        let err = perf_patch(&json!({ "perf_signals": "Checkout p95 histogram_quantile(0.95, …)" })).unwrap_err();
        assert!(err.starts_with("Line 1 isn't a signal") && err.ends_with('.'), "{err}");
        assert_eq!(perf_patch(&json!({ "perf_signals": "" })), Ok(json!({ "perf_signals": "" })), "clearing them is allowed");
        assert_eq!(perf_patch(&json!({})), Ok(json!({})));

        let dir = std::env::temp_dir().join(format!("su-perf-api-{}", crate::core::inbox::short_id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut settings = crate::core::settings::Settings::default();
        settings.apply_patch(&perf_patch(&json!({ "perf_signals": good })).unwrap());
        let status = perf_status(&settings, &dir);
        assert_eq!((status["count"].clone(), status["problems"].clone(), status["routine"].clone()), (json!(2), json!([]), json!(false)));
        let perf = |d: &std::path::Path| add_routine(d, crate::core::perf::ASSISTANT, crate::core::perf::ROUTINE_TITLE, crate::core::perf::ROUTINE_CRON, crate::core::perf::ROUTINE_INSTRUCTIONS);
        assert_eq!(perf(&dir)["created"], true);
        assert_eq!(perf(&dir)["created"], false, "added once");
        let task = &crate::core::automation::AutomationStore::load(&dir).list()[0];
        assert_eq!((task.agent.as_str(), task.schedule.cron.as_deref()), ("perf", Some("30 9 * * 1")));
        assert_eq!(perf_status(&settings, &dir)["routine"], true);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Bug Reproduction's route takes a staging address and nothing else, refuses one the
    /// browser could never reach, and says what the browser still needs to reach it.
    #[test]
    pub(super) fn the_bug_repro_route_takes_only_a_reachable_staging_address() {
        assert_eq!(
            repro_patch(&json!({ "repro_staging": "https://staging.example.com", "browser_enabled": true })),
            Ok(json!({ "repro_staging": "https://staging.example.com" })),
            "the browser switch stays on the Browser page"
        );
        assert_eq!(repro_patch(&json!({ "repro_staging": "" })), Ok(json!({ "repro_staging": "" })), "clearing it is allowed");
        assert!(repro_patch(&json!({ "repro_staging": "http://169.254.169.254/latest" })).unwrap_err().contains("never reaches"));
        assert!(repro_patch(&json!({ "repro_staging": "ftp://files.example.com" })).is_err());
        assert_eq!(repro_patch(&json!({ "repro_staging": 3 })), Ok(json!({})));

        let mut settings = crate::core::settings::Settings::default();
        settings.apply_patch(&repro_patch(&json!({ "repro_staging": "  http://localhost:3000  " })).unwrap());
        let status = repro_status(&settings);
        assert_eq!(status["staging"], "http://localhost:3000");
        assert_eq!((status["on_this_mac"].clone(), status["browser"].clone(), status["loopback"].clone()), (json!(true), json!(false), json!(false)));
        settings.repro_staging = "https://staging.example.com".into();
        assert_eq!(repro_status(&settings)["on_this_mac"], false);
    }

    /// The email bot's route changes its own two settings and nothing else.
    #[test]
    pub(super) fn the_email_bot_route_writes_only_its_own_settings() {
        let patch = email_bot_patch(&json!({
            "email_inbound": true,
            "email_allowed": "ada@example.com",
            "telegram_inbound": true,
            "ollama_model": "evil",
        }));
        assert_eq!(patch, json!({ "email_inbound": true, "email_allowed": "ada@example.com" }));
        assert_eq!(email_bot_patch(&json!({ "email_inbound": "yes", "email_allowed": 5 })), json!({}), "wrong types change nothing");

        let dir = std::env::temp_dir().join(format!("su-emailbot-api-{}", crate::core::inbox::short_id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut s = crate::core::settings::Settings::default();
        s.apply_patch(&patch);
        let status = email_bot_status(&s, &dir);
        assert_eq!(status["inbound"], true);
        assert_eq!(status["allowed_count"], 1);
        assert_eq!(status["problem"], Value::Null, "the shipped Email Assistant can draft");
        assert_eq!(status["daily_cap"], crate::core::email_bot::DAILY_CAP);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    pub(super) fn a_brand_new_conversation_offers_its_own_assistants_skills_and_not_everything() {
        // A session reaches disk on its FIRST TURN, so every freshly-opened one was
        // unknown here — and "unknown" and "this assistant was given no list" were
        // spelled the same way: an empty vec, which downstream means "everything
        // installed". Pick Customer Support, press `/`, and the menu offered all
        // thirty-eight procedures; it narrowed only once you had sent a message,
        // which is after the menu stops mattering.
        let d = std::env::temp_dir().join(format!("allowed-skills-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();

        // No stored session, and the client says which assistant it is showing.
        let support = allowed_skills_for(&d, "brand-new", Some("support"));
        assert_eq!(support, vec!["support-triage", "help-article", "bug-report"]);
        let finance = allowed_skills_for(&d, "brand-new", Some("finance"));
        assert!(finance.contains(&"runway-model".to_string()));
        assert!(!finance.contains(&"support-triage".to_string()), "a roster is a narrowing, not a suggestion");

        // A generalist genuinely has no list, and empty still means "everything".
        assert!(allowed_skills_for(&d, "brand-new", Some("assistant")).is_empty());
        // And an agent nobody has heard of narrows to nothing rather than widening.
        assert!(allowed_skills_for(&d, "brand-new", Some("no-such-assistant")).is_empty());
        // No hint at all is the old behaviour, kept for callers that have a real session.
        assert!(allowed_skills_for(&d, "brand-new", None).is_empty());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[tokio::test]
    pub(super) async fn install_is_a_route_and_not_an_assistant_called_install() {
        // `/v1/personas/:id` matches anything, so before the installer existed a
        // POST to `/v1/personas/install` landed in `persona_update` as an assistant
        // named "install" — and was told it succeeded. Two handlers both match this
        // path; the static segment has to win, and route order or a rename could
        // quietly change that at any time.
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use tower::ServiceExt;

        async fn installer() -> &'static str {
            "installer"
        }
        async fn updater(axum::extract::Path(id): axum::extract::Path<String>) -> String {
            format!("updater:{id}")
        }
        // Registered in the same order, and with the same shapes, as the real table.
        let app = Router::new()
            .route("/v1/personas/install", post(installer))
            .route("/v1/personas/:id", get(updater).post(updater).patch(updater).delete(updater));

        let res = app
            .clone()
            .oneshot(Request::post("/v1/personas/install").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = axum::body::to_bytes(res.into_body(), 64).await.unwrap();
        assert_eq!(&body[..], b"installer", "the wildcard swallowed the install route");

        // And the wildcard still serves everything else.
        let res = app
            .oneshot(Request::post("/v1/personas/acme-ops").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let body = axum::body::to_bytes(res.into_body(), 64).await.unwrap();
        assert_eq!(&body[..], b"updater:acme-ops");
    }
