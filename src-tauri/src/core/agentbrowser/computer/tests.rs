use super::*;

    #[test]
    fn only_plain_ids_become_directories() {
        for ok in ["assistant", "coder", "my-helper_2", "A1"] {
            assert!(is_plain_id(ok), "{ok}");
        }
        for bad in ["", "../x", "-leading", "a/b", "a b", ".hidden", &"x".repeat(65)] {
            assert!(!is_plain_id(bad), "{bad}");
        }
        assert!(is_frame_id("f-18f2a3b4c5d-0a1b"));
        assert!(!is_frame_id("../../secrets"));
        assert!(!is_frame_id("f-a/b"));
    }

    /// Two assistants starting their browsers at once each held their own lock and waited
    /// for the other's, forever. Making room reads the others without waiting for anyone.
    #[tokio::test]
    async fn making_room_for_a_browser_waits_for_no_other_browser() {
        let dir = std::env::temp_dir().join(format!("su-computer-room-{}", std::process::id()));
        let other = get(&dir, "room-other").unwrap();
        // Another browser mid-start or mid-action: its lock is held.
        let _held = other.running.lock().await;
        tokio::time::timeout(Duration::from_secs(2), async { make_room("room-mine") })
            .await
            .expect("making room waited on another browser's lock");
    }

    #[test]
    fn a_bug_report_gets_the_errors_and_not_the_chatter() {
        let console = |kind: &str, args: Value| note_of("Runtime.consoleAPICalled", &json!({"type": kind, "args": args}));
        assert_eq!(console("error", json!([{"type": "string", "value": "Cart total is NaN"}])).as_deref(), Some("console error: Cart total is NaN"));
        assert_eq!(console("assert", json!([{"type": "string", "value": "expected items"}])).as_deref(), Some("console error: expected items"));
        assert_eq!(console("warning", json!([{"type": "number", "value": 3}, {"type": "string", "value": "retries"}])).as_deref(), Some("console warning: 3 retries"));
        assert_eq!(console("error", json!([{"type": "object", "description": "TypeError: x is undefined"}])).as_deref(), Some("console error: TypeError: x is undefined"));
        assert_eq!(console("log", json!([{"type": "string", "value": "rendered"}])), None, "a log is chatter");
        assert_eq!(console("info", json!([])), None);

        let thrown = note_of(
            "Runtime.exceptionThrown",
            &json!({"exceptionDetails": {"text": "Uncaught", "url": "https://staging.example.com/app.js", "lineNumber": 41,
                     "exception": {"description": "TypeError: Cannot read properties of null (reading 'total')\n    at render (app.js:42:7)"}}}),
        );
        assert_eq!(
            thrown.as_deref(),
            Some("uncaught exception: TypeError: Cannot read properties of null (reading 'total') at https://staging.example.com/app.js:42"),
            "the first line, and the line numbered as an editor numbers it"
        );

        let log = |level: &str, source: &str| {
            note_of("Log.entryAdded", &json!({"entry": {"level": level, "source": source, "text": "Failed to load resource: 500", "url": "https://staging.example.com/api/cart"}}))
        };
        assert_eq!(log("error", "network").as_deref(), Some("network error: Failed to load resource: 500 — https://staging.example.com/api/cart"));
        assert!(log("warning", "security").is_some());
        assert_eq!(log("error", "javascript"), None, "console messages come from Runtime, so none is counted twice");
        assert_eq!(log("info", "network"), None);
        assert_eq!(note_of("Page.loadEventFired", &json!({})), None);
    }

    #[test]
    fn a_page_cannot_flood_the_report_or_break_its_lines() {
        let long = format!("line one\nline two\u{7}{}", "x".repeat(1000));
        let note = note_of("Runtime.consoleAPICalled", &json!({"type": "error", "args": [{"type": "string", "value": long}]})).unwrap();
        assert!(!note.contains('\n') && !note.contains('\u{7}'), "{note}");
        assert!(note.starts_with("console error: line one line two"));
        assert_eq!(note.chars().count(), "console error: ".len() + 300);
    }

    #[test]
    fn a_cdp_error_about_a_vanished_node_tells_the_model_to_resnapshot() {
        assert!(words("No node with given id found".into()).contains("Take a new snapshot"));
        assert!(words(super::super::cdp::CLOSED.into()).contains("browser_navigate"));
        assert_eq!(words("something else".into()), "something else");
    }

    #[tokio::test]
    async fn stale_and_unknown_refs_are_refused_before_any_browser_is_touched() {
        let dir = std::env::temp_dir().join(format!("su-computer-refs-{}", std::process::id()));
        let c = get(&dir, "refs-test").unwrap();
        assert!(c.resolve("e1", 1).unwrap_err().contains("Take a snapshot first"));
        {
            let mut r = c.shared.refs.lock().unwrap();
            r.generation = 5;
            r.snapshot = 5;
            r.elements = vec![Element {
                r#ref: "e1".into(),
                role: "button".into(),
                name: "Submit order".into(),
                value: None,
                disabled: false,
                checked: None,
                input_type: String::new(),
                backend_node_id: 9,
            }];
            r.url = "https://shop.example.com/cart".into();
        }
        assert_eq!(c.resolve("e1", 5).unwrap().name, "Submit order");
        assert!(c.resolve("e1", 3).unwrap_err().contains("snapshot 3 and the page is now at 5"));
        assert!(c.resolve("e9", 5).unwrap_err().contains("No element here carries the ref e9"));
        // The gateway's view of the same element.
        let (el, url) = element_for("refs-test", "e1").unwrap();
        assert_eq!((el.name.as_str(), url.as_str()), ("Submit order", "https://shop.example.com/cart"));

        // While the user drives, acting is refused before anything else.
        c.take_control();
        assert!(c.click("e1", 5, None).await.unwrap_err().contains("The user has control"));
        c.release_control();
        let _ = std::fs::remove_dir_all(&dir);
    }
