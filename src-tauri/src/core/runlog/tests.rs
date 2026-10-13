use super::*;

    fn scratch() -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(0);
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let p = std::env::temp_dir().join(format!("su-runlog-{}-{}", nanos, N.fetch_add(1, Ordering::SeqCst)));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn frame(kind: &str, text: &str) -> Value {
        json!({ "type": kind, "data": { "text": text } })
    }

    #[test]
    fn a_finished_run_can_still_be_read_back() {
        let d = scratch();
        start(&d);
        append(&d, &frame("turn_start", ""));
        append(&d, &frame("tool_started", "run_shell"));
        append(&d, &frame("assistant_message", "done"));

        let page = read(&d, 0, 100);
        assert_eq!(page.events.len(), 3);
        assert_eq!(page.events[0]["type"], "turn_start");
        assert_eq!(page.events[1]["data"]["text"], "run_shell");
        // Every frame is addressable by its position.
        assert_eq!(page.events[2]["seq"], 2);
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn offsets_are_stable_so_a_reader_can_resume() {
        let d = scratch();
        start(&d);
        for i in 0..5 {
            append(&d, &frame("tool_started", &format!("step {i}")));
        }
        let first = read(&d, 0, 2);
        assert_eq!(first.events.len(), 2);
        assert_eq!(first.next_offset, 2);

        // More arrives while the caller was away; resuming picks up exactly there.
        append(&d, &frame("assistant_message", "finished"));
        let rest = read(&d, first.next_offset, 100);
        assert_eq!(rest.events.len(), 4);
        assert_eq!(rest.events[0]["data"]["text"], "step 2");
        assert_eq!(rest.events[3]["data"]["text"], "finished");
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn the_token_stream_is_not_journalled() {
        // Thousands of deltas per turn, reconstructing text assistant_message
        // already carries. The journal is an account, not a recording.
        let d = scratch();
        start(&d);
        append(&d, &frame("assistant_delta", "he"));
        append(&d, &frame("assistant_delta", "llo"));
        append(&d, &frame("assistant_message", "hello"));
        let page = read(&d, 0, 100);
        assert_eq!(page.events.len(), 1);
        assert_eq!(page.events[0]["type"], "assistant_message");
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn credentials_never_reach_the_file() {
        let d = scratch();
        start(&d);
        append(&d, &frame("tool_started", "curl -H 'Authorization: Bearer eyJhbGciOiJIUzI1NiJ9.secret'"));
        let raw = std::fs::read_to_string(path(&d)).unwrap();
        assert!(!raw.contains("eyJhbGciOiJIUzI1NiJ9"), "the token is on disk: {raw}");
        assert!(raw.contains("[REDACTED]"));
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn a_new_turn_replaces_the_last_ones_journal() {
        let d = scratch();
        start(&d);
        append(&d, &frame("assistant_message", "first turn"));
        start(&d);
        append(&d, &frame("assistant_message", "second turn"));
        let page = read(&d, 0, 100);
        assert_eq!(page.events.len(), 1);
        assert_eq!(page.events[0]["data"]["text"], "second turn");
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn reading_a_run_that_never_wrote_one_is_empty_not_an_error() {
        let d = scratch();
        let page = read(&d, 0, 100);
        assert!(page.events.is_empty());
        assert_eq!(len(&d), 0);
        std::fs::remove_dir_all(&d).ok();
    }
