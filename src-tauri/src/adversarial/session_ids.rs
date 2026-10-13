use crate::core::sessions::{Message, SessionStore};

    #[test]
    fn two_different_ids_never_share_a_conversation() {
        // dir() maps every non-alphanumeric to '_', so ids differing only in
        // punctuation collapse onto one folder. The second id then *loads the
        // first one's transcript* — a cross-conversation leak, not just a clash.
        let d = super::scratch("sess-collide");
        let store = SessionStore::new(&d);
        let a = store.get_or_create("chat/one", "persona", "", "m");
        store.append(&a.session_id, Message::new("user", "my private thing"));
        let b = store.get_or_create("chat:one", "persona", "", "m");
        let leaked = b.messages.iter().any(|m| m.content.contains("my private thing"));
        std::fs::remove_dir_all(&d).ok();
        assert!(
            !leaked && b.session_id != a.session_id,
            "id 'chat:one' loaded the transcript belonging to 'chat/one' — distinct ids share a directory"
        );
    }

    #[test]
    fn a_session_written_under_the_old_lossy_name_is_still_found() {
        // The injective rename must not strand conversations from earlier installs.
        let d = super::scratch("sess-legacy");
        let store = SessionStore::new(&d);
        let legacy = d.join("sessions").join("chat_one");
        std::fs::create_dir_all(&legacy).unwrap();
        let s = store.get_or_create("placeholder", "persona", "", "m");
        let mut s = s;
        s.session_id = "chat/one".into();
        s.messages.push(Message::new("user", "from the old folder"));
        std::fs::write(legacy.join("session.json"), serde_json::to_string(&s).unwrap()).unwrap();

        let back = store.get("chat/one");
        // …and an id that merely collides with that folder must NOT adopt it.
        let impostor = store.get("chat:one");
        std::fs::remove_dir_all(&d).ok();
        assert!(
            back.is_some_and(|b| b.messages.iter().any(|m| m.content.contains("from the old folder"))),
            "a pre-existing session under the legacy folder name was lost"
        );
        assert!(impostor.is_none(), "a colliding id adopted the legacy folder");
    }

    #[test]
    fn deleting_a_degenerate_id_does_not_wipe_every_session() {
        // delete() -> remove_dir_all(dir(id)); dir("") is the store root itself.
        let d = super::scratch("sess-nuke");
        let store = SessionStore::new(&d);
        store.get_or_create("keep-me", "persona", "", "m");
        store.delete("");
        let survived = store.get("keep-me").is_some();
        std::fs::remove_dir_all(&d).ok();
        assert!(survived, "delete(\"\") removed the entire sessions store");
    }
