use super::*;

    fn tmp() -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(0);
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let p = std::env::temp_dir().join(format!("bba-chats-{}-{}", nanos, N.fetch_add(1, Ordering::SeqCst)));
        std::fs::create_dir_all(&p).unwrap();
        p
    }
    fn msg(role: &str, c: &str) -> StoredMessage { StoredMessage { role: role.into(), content: c.into() } }

    #[test]
    fn save_get_list_roundtrip() {
        let d = tmp();
        let id = save(&d, None, None, vec![msg("user", "What is Apollo?"), msg("assistant", "A project.")]);
        assert!(id.starts_with("chat-"));
        let got = get(&d, &id).unwrap();
        assert_eq!(got.title, "What is Apollo?"); // derived from first user message
        assert_eq!(got.messages.len(), 2);
        let summaries = list(&d);
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].message_count, 2);
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn save_updates_existing_by_id() {
        let d = tmp();
        let id = save(&d, None, None, vec![msg("user", "hi")]);
        save(&d, Some(id.clone()), Some("Renamed".into()), vec![msg("user", "hi"), msg("assistant", "hello")]);
        let got = get(&d, &id).unwrap();
        assert_eq!(got.title, "Renamed");
        assert_eq!(got.messages.len(), 2);
        assert_eq!(list(&d).len(), 1, "updated, not duplicated");
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn list_sorted_by_recency() {
        let d = tmp();
        let a = save(&d, None, Some("first".into()), vec![msg("user", "a")]);
        std::thread::sleep(std::time::Duration::from_millis(2));
        let b = save(&d, None, Some("second".into()), vec![msg("user", "b")]);
        let l = list(&d);
        assert_eq!(l[0].id, b);
        assert_eq!(l[1].id, a);
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn title_derivation_handles_empty() {
        assert_eq!(derive_title(&[]), "New chat");
        assert_eq!(derive_title(&[msg("assistant", "hi")]), "New chat");
        assert_eq!(derive_title(&[msg("user", "Hello there")]), "Hello there");
    }
