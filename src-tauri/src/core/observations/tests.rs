use super::*;

    use std::sync::atomic::{AtomicUsize, Ordering};

    /// A scratch data dir, same shape the rest of the tree uses for this.
    fn scratch() -> PathBuf {
        static N: AtomicUsize = AtomicUsize::new(0);
        let d = std::env::temp_dir().join(format!(
            "su-obs-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        let _ = fs::create_dir_all(&d);
        d
    }

    fn store() -> ObservationStore {
        ObservationStore::for_session(&scratch(), "sess-1")
    }

    #[test]
    fn a_handle_is_the_content_so_the_same_output_stores_once() {
        let s = store();
        let a = s.put("the same ninety seconds of build output").unwrap();
        let b = s.put("the same ninety seconds of build output").unwrap();
        assert_eq!(a, b, "identical text must share a handle");
        let n = fs::read_dir(s.dir()).unwrap().count();
        assert_eq!(n, 1, "and must be stored once, not twice");
    }

    #[test]
    fn a_recall_returns_what_was_seen_not_what_is_there_now() {
        // The whole reason the handle is content-addressed rather than a path: the
        // source can change or disappear, and the model asked for what it read.
        let s = store();
        let old = s.put("metrics host=localhost port=8080").unwrap();
        let new = s.put("metrics host=localhost port=9000").unwrap();
        assert_ne!(old, new);
        assert_eq!(s.get(&old).unwrap(), "metrics host=localhost port=8080");
    }

    #[test]
    fn a_slice_is_characters_and_reports_the_whole_length() {
        let s = store();
        let id = s.put("héllo wörld, the middle is here").unwrap();
        let (out, total) = s.slice(&id, 6, 5).unwrap();
        assert_eq!(out, "wörld");
        assert_eq!(total, "héllo wörld, the middle is here".chars().count());
    }

    #[test]
    fn a_range_past_the_end_is_an_answer_not_a_failure() {
        let s = store();
        let id = s.put("short").unwrap();
        let (out, total) = s.slice(&id, 900, 10).unwrap();
        assert_eq!(out, "");
        assert_eq!(total, 5, "still tells the model how long it actually is");
    }

    #[test]
    fn a_handle_that_is_not_a_handle_never_reaches_the_filesystem() {
        let s = store();
        for bad in ["../../secrets", "abc", "", "../etc/passwd", "zzzzzzzzzzzz"] {
            assert!(s.get(bad).is_none(), "{bad:?} must not resolve");
        }
    }

    #[test]
    fn the_cap_refuses_rather_than_evicting_a_live_handle() {
        // Refusing degrades one notice to the old wording. Evicting would hand the
        // model a handle that resolves to nothing, which is worse than never
        // having offered one.
        let s = store().with_cap(64);
        let big = "x".repeat(65);
        assert!(s.put(&big).is_none(), "a put over the cap is refused");
        let small = s.put("still works afterwards").unwrap();
        assert_eq!(s.get(&small).unwrap(), "still works afterwards");
    }

    #[test]
    fn two_sessions_never_share_a_store_even_with_punctuated_ids() {
        // `chat/one` and `chat:one` both used to map to `chat_one`. Sharing a
        // store would leak whole command outputs between conversations.
        let td = scratch();
        let a = ObservationStore::for_session(&td, "chat/one");
        let b = ObservationStore::for_session(&td, "chat:one");
        assert_ne!(a.dir(), b.dir());
        let id = a.put("only session a saw this").unwrap();
        assert!(b.get(&id).is_none(), "b must not see a's observation");
    }

    #[test]
    fn clearing_a_session_takes_its_observations_with_it() {
        let s = store();
        let id = s.put("dies with the session").unwrap();
        s.clear();
        assert!(s.get(&id).is_none());
    }
