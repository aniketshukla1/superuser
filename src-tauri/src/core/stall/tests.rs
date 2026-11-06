use super::*;

    #[test]
    fn durations_read_like_speech() {
        assert_eq!(in_words(Duration::from_secs(60)), "a minute");
        assert_eq!(in_words(Duration::from_millis(60_250)), "a minute");
        assert_eq!(in_words(Duration::from_secs(5)), "5 seconds");
        assert_eq!(in_words(Duration::from_secs(300)), "5 minutes");
        assert_eq!(in_words(Duration::from_secs(95)), "a minute and a half");
    }

    #[test]
    fn the_sentence_says_what_happened_and_what_to_try() {
        let s = sentence("gemma4:12b", Duration::from_secs(300), true);
        assert!(s.starts_with("gemma4:12b went quiet:"), "{s}");
        assert!(s.contains("5 minutes") && s.contains("Ollama"), "{s}");
        assert!(sentence("claude-opus-5", Duration::from_secs(60), false).contains("provider is reachable"));
    }

    #[tokio::test]
    async fn silence_ends_the_wait_and_arrival_does_not() {
        let cancel = AtomicBool::new(false);
        let quiet = within(std::future::pending::<()>(), Some(Duration::from_millis(600)), &cancel).await;
        assert!(matches!(quiet, Err(Quiet::Stalled(d)) if d >= Duration::from_millis(600)));

        let prompt = within(async { 7 }, Some(Duration::from_millis(600)), &cancel).await;
        assert_eq!(prompt, Ok(7));

        // A slow answer that does arrive inside the limit is an answer.
        let slow = within(
            async {
                tokio::time::sleep(Duration::from_millis(400)).await;
                "late"
            },
            Some(Duration::from_millis(900)),
            &cancel,
        )
        .await;
        assert_eq!(slow, Ok("late"));
    }

    #[tokio::test]
    async fn stop_ends_a_silent_wait_too() {
        let cancel = std::sync::Arc::new(AtomicBool::new(false));
        let c = cancel.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(300)).await;
            c.store(true, Ordering::SeqCst);
        });
        let r = within(std::future::pending::<()>(), None, &cancel).await;
        assert_eq!(r, Err(Quiet::Stopped));
    }

    #[test]
    fn a_stalled_turn_ends_with_the_sentence_and_exactly_one_row() {
        // `stalled_model_ends_turn`, at the level it can be checked
        // without a model: a stall is a property of the harness, and the provider
        // fakes that go quiet are in `llm::tests` and `commands::stream_tests`.
        let dir = std::env::temp_dir().join(format!("su-stall-{}-{}", std::process::id(), line!()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let said = end_turn(&dir, "__run__run-7", "assistant", "gemma4:12b", Duration::from_secs(300), true);
        assert!(said.starts_with("gemma4:12b went quiet:"), "{said}");
        let rows = crate::core::audit::recent(&dir, 10);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].event_name(), "turn.stalled");
        assert!(!rows[0].ok);
        assert_eq!(rows[0].initiator.as_ref().unwrap().kind, "schedule", "and it says nobody was watching");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_limit_of_zero_switches_the_watchdog_off() {
        set_limits(0, 5);
        assert_eq!(limit(false), None);
        assert_eq!(limit(true), Some(Duration::from_secs(5)));
        set_limits(DEFAULT_CLOUD_SECS, DEFAULT_LOCAL_SECS);
    }
