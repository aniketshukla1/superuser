use super::*;

    pub(super) fn scratch() -> PathBuf {
        static SCRATCH_N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let seq = SCRATCH_N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let n = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let d = std::env::temp_dir().join(format!("bba-sessions-{}-{}-{}", std::process::id(), n, seq));
        let _ = fs::create_dir_all(&d);
        d
    }

    #[test]
    pub(super) fn a_source_the_user_never_settled_keeps_the_reach_it_always_had() {
        // The gate is opt-in. Before scopes existed every connected source could be
        // sent through, and automations were built on that — so "no grant recorded"
        // must read as "not narrowed", never as "read-only".
        let dir = scratch();
        let store = SessionStore::new(&dir);
        let s = Session::new("assistant", "/tmp/ws", "llama3.2");
        store.save(&s);
        assert_eq!(store.may_write(&s.session_id, "slack"), None);
    }

    #[test]
    pub(super) fn marking_a_source_read_only_blocks_writing_through_it() {
        let dir = scratch();
        let store = SessionStore::new(&dir);
        let s = Session::new("assistant", "/tmp/ws", "llama3.2");
        store.save(&s);
        store.set_connection(&s.session_id, "slack", None, Some("read"), false);
        assert_eq!(store.may_write(&s.session_id, "slack"), Some(false));
        store.set_connection(&s.session_id, "slack", None, Some("write"), false);
        assert_eq!(store.may_write(&s.session_id, "slack"), Some(true));
    }

    #[test]
    pub(super) fn muting_a_source_does_not_reset_the_scope_chosen_for_it() {
        // The two controls move independently: toggling a source off and back on
        // must not silently widen it again.
        let dir = scratch();
        let store = SessionStore::new(&dir);
        let s = Session::new("assistant", "/tmp/ws", "llama3.2");
        store.save(&s);
        store.set_connection(&s.session_id, "slack", None, Some("read"), false);
        store.set_connection(&s.session_id, "slack", Some(false), None, false);
        store.set_connection(&s.session_id, "slack", Some(true), None, false);
        let g = store.get(&s.session_id).unwrap().connections.get("slack").cloned().unwrap();
        assert_eq!(g.scope, SCOPE_READ);
        assert!(g.enabled);
    }

    #[test]
    pub(super) fn a_muted_source_cannot_be_written_through_whatever_its_scope_says() {
        let dir = scratch();
        let store = SessionStore::new(&dir);
        let s = Session::new("assistant", "/tmp/ws", "llama3.2");
        store.save(&s);
        store.set_connection(&s.session_id, "slack", Some(false), Some("write"), false);
        assert_eq!(store.may_write(&s.session_id, "slack"), Some(false));
    }

    #[test]
    pub(super) fn clearing_a_grant_returns_the_session_to_inheriting_the_default() {
        let dir = scratch();
        let store = SessionStore::new(&dir);
        let s = Session::new("assistant", "/tmp/ws", "llama3.2");
        store.save(&s);
        store.set_connection(&s.session_id, "slack", None, Some("read"), false);
        store.set_connection(&s.session_id, "slack", None, None, true);
        assert_eq!(store.may_write(&s.session_id, "slack"), None);
    }

    #[test]
    pub(super) fn an_unrecognised_scope_never_widens_access() {
        // A permission string we don't understand must fall to the safe end.
        assert_eq!(normalize_scope("WRITE"), SCOPE_WRITE);
        assert_eq!(normalize_scope("read"), SCOPE_READ);
        assert_eq!(normalize_scope("admin"), SCOPE_READ);
        assert_eq!(normalize_scope(""), SCOPE_READ);
    }

    #[test]
    pub(super) fn a_grant_survives_a_reload_from_disk() {
        let dir = scratch();
        let s = Session::new("assistant", "/tmp/ws", "llama3.2");
        SessionStore::new(&dir).save(&s);
        SessionStore::new(&dir).set_connection(&s.session_id, "telegram", None, Some("read"), false);
        // A fresh store reads the same file the running app would.
        assert_eq!(SessionStore::new(&dir).may_write(&s.session_id, "telegram"), Some(false));
    }

    #[test]
    pub(super) fn a_scheduled_answer_lands_in_the_conversation_it_came_from() {
        let dir = scratch();
        let store = SessionStore::new(&dir);
        let s = Session::new("assistant", "/tmp/ws", "llama3.2");
        store.save(&s);
        store.append(&s.session_id, Message::new("user", "remind me about standup every morning"));
        store.append(&s.session_id, Message::new("assistant", "Scheduled it."));

        assert!(store.inject_scheduled(&s.session_id, "task-1", "Standup reminder", "Standup is at 09:30.", 3));
        let back = store.get(&s.session_id).unwrap();
        assert_eq!(back.messages.len(), 4);
        assert_eq!(back.messages[2].content, "[scheduled] Standup reminder");
        assert_eq!(back.messages[3].content, "Standup is at 09:30.");
        // Tagged, so the pruner can tell injected pairs from the user's own turns.
        assert_eq!(back.messages[3].items[0]["kind"], "scheduled");
        // And it reaches the model as ordinary history, so "expand on that" works.
        assert!(back.history().iter().any(|(_, c)| c == "Standup is at 09:30."));

        assert!(!store.inject_scheduled("no-such-session", "task-1", "t", "a", 3));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    pub(super) fn a_frequent_task_cannot_bury_the_users_own_messages() {
        let dir = scratch();
        let store = SessionStore::new(&dir);
        let s = Session::new("assistant", "/tmp/ws", "llama3.2");
        store.save(&s);
        store.append(&s.session_id, Message::new("user", "watch the build every 5 minutes"));
        store.append(&s.session_id, Message::new("assistant", "Watching."));

        for i in 0..10 {
            store.inject_scheduled(&s.session_id, "task-1", "Build watch", &format!("run {i}"), 3);
        }
        let back = store.get(&s.session_id).unwrap();
        let injected: Vec<_> = back.messages.iter().filter(|m| m.items.iter().any(|i| i["kind"] == "scheduled")).collect();
        assert_eq!(injected.len(), 6, "three pairs kept");
        assert_eq!(injected.last().unwrap().content, "run 9", "the newest survives");
        assert!(!injected.iter().any(|m| m.content == "run 0"), "the oldest is pruned");
        // The user's own turns are never touched.
        assert_eq!(back.messages[0].content, "watch the build every 5 minutes");
        assert_eq!(back.messages[1].content, "Watching.");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    pub(super) fn the_wire_transcript_is_kept_on_disk_and_never_sent_to_a_client() {
        // Two different audiences. The next model needs the tool traffic; the webview
        // cannot draw it and would receive hundreds of kilobytes of it per session.
        let mut m = Message::new("assistant", "done");
        m.wire = Some(crate::core::transcript::TurnWire {
            shape: crate::core::transcript::WireShape::Ollama,
            model: "llama3.2".into(),
            messages: vec![serde_json::json!({ "role": "tool", "content": "big" })],
        });
        assert!(m.wire.is_some(), "stored");
        assert!(m.without_wire().wire.is_none(), "not shipped");
        // And the field is absent from the JSON entirely rather than sent as null.
        let ui = serde_json::to_string(&m.without_wire()).unwrap();
        assert!(!ui.contains("wire"), "got: {ui}");
    }

    #[test]
    pub(super) fn a_session_written_before_transcripts_existed_still_loads() {
        // Every session on disk today has no `wire` key. Reading one must not fail,
        // and must not invent an empty transcript that replays as a turn with no
        // tool calls.
        let raw = r#"{"role":"assistant","content":"hi","at":1.0}"#;
        let m: Message = serde_json::from_str(raw).unwrap();
        assert_eq!(m.content, "hi");
        assert!(m.wire.is_none());
    }

    #[test]
    pub(super) fn history_is_the_whole_transcript_until_something_is_compacted() {
        let mut s = Session::new("assistant", "/tmp/ws", "llama3.2");
        for i in 0..3 {
            s.messages.push(Message::new("user", format!("q{i}")));
            s.messages.push(Message::new("assistant", format!("a{i}")));
        }
        assert_eq!(s.history().len(), 6);

        s.compact_from = 4;
        s.compact_summary = "The user asked two questions about Q3.".into();
        let h = s.history();
        // summary pair + the two messages after the split
        assert_eq!(h.len(), 4);
        assert!(h[0].1.contains("The user asked two questions about Q3."));
        assert_eq!(h[0].0, "user", "the summary is framed as context, not as my own words");
        assert_eq!(h[2].1, "q2");
        assert_eq!(h[3].1, "a2");
    }
