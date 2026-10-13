use super::*;

    #[test]
    pub(super) fn compaction_never_touches_the_visible_transcript() {
        let dir = scratch();
        let store = SessionStore::new(&dir);
        let s = Session::new("assistant", "/tmp/ws", "llama3.2");
        store.save(&s);
        for i in 0..4 {
            store.append(&s.session_id, Message::new("user", format!("q{i}")));
            store.append(&s.session_id, Message::new("assistant", format!("a{i}")));
        }
        assert!(store.set_compaction(&s.session_id, 4, "earlier: two questions"));

        let back = store.get(&s.session_id).unwrap();
        assert_eq!(back.messages.len(), 8, "every message still on screen");
        assert_eq!(back.compact_from, 4);
        assert_eq!(back.history().len(), 6, "but only 4 replayed, behind a summary");

        // Compacting back to zero restores the full history — it is reversible.
        store.set_compaction(&s.session_id, 0, "");
        assert_eq!(store.get(&s.session_id).unwrap().history().len(), 8);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    pub(super) fn a_compaction_point_past_the_end_is_clamped() {
        let dir = scratch();
        let store = SessionStore::new(&dir);
        let s = Session::new("assistant", "/tmp/ws", "llama3.2");
        store.save(&s);
        store.append(&s.session_id, Message::new("user", "hi"));
        store.set_compaction(&s.session_id, 99, "everything");
        let back = store.get(&s.session_id).unwrap();
        assert_eq!(back.compact_from, 1);
        assert_eq!(back.history().len(), 2, "summary pair only — no panic, no lost message");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    pub(super) fn sessions_written_before_compaction_existed_still_load() {
        let old = r#"{"session_id":"s-1","agent":"assistant","messages":[{"role":"user","content":"hi"}]}"#;
        let s: Session = serde_json::from_str(old).expect("old session still deserializes");
        assert_eq!(s.compact_from, 0);
        assert!(s.compact_summary.is_empty());
        assert_eq!(s.history().len(), 1);
    }

    #[test]
    pub(super) fn create_append_and_reload() {
        let dir = scratch();
        let store = SessionStore::new(&dir);
        let s = Session::new("assistant", "/tmp/ws", "llama3.2");
        store.save(&s);

        store.append(&s.session_id, Message::new("user", "Write me a brief on Q3")).unwrap();
        store.append(&s.session_id, Message::new("assistant", "Done — brief.md")).unwrap();

        let back = store.get(&s.session_id).unwrap();
        assert_eq!(back.messages.len(), 2);
        assert_eq!(back.agent, "assistant");
        // title derives from the first user message
        assert_eq!(back.derived_title(), "Write me a brief on Q3");
        assert!(back.updated_at >= back.created_at);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    pub(super) fn list_is_newest_first_and_delete_removes_work() {
        let dir = scratch();
        let store = SessionStore::new(&dir);
        let a = Session::new("assistant", "", "m");
        store.save(&a);
        std::thread::sleep(std::time::Duration::from_millis(5));
        let b = Session::new("ops", "", "m");
        store.save(&b);
        store.append(&b.session_id, Message::new("user", "later")).unwrap();

        let list = store.list();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].session_id, b.session_id, "newest first");

        // the working folder lives inside the session dir, so delete takes it too
        let work = store.workdir(&a.session_id);
        fs::write(work.join("out.md"), "x").unwrap();
        assert!(work.exists());
        assert!(store.delete(&a.session_id));
        assert!(!work.exists());
        assert_eq!(store.list().len(), 1);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    pub(super) fn rename_flags_and_get_or_create() {
        let dir = scratch();
        let store = SessionStore::new(&dir);

        // get_or_create makes one on demand (the GUI picks the id, then connects)
        let s = store.get_or_create("s-chosen", "ops", "/w", "m");
        assert_eq!(s.session_id, "s-chosen");
        assert!(store.get("s-chosen").is_some());
        // …and is idempotent
        store.append("s-chosen", Message::new("user", "hi")).unwrap();
        let again = store.get_or_create("s-chosen", "ops", "/w", "m");
        assert_eq!(again.messages.len(), 1);

        assert!(store.rename("s-chosen", "Incident review"));
        assert_eq!(store.get("s-chosen").unwrap().derived_title(), "Incident review");

        assert!(store.set_flags("s-chosen", Some(true), None));
        let s2 = store.get("s-chosen").unwrap();
        assert!(s2.pinned && !s2.archived);

        assert!(!store.rename("ghost", "x"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    pub(super) fn titles_skip_filler_and_name_the_task() {
        let t = |text: &str| {
            let mut s = Session::new("assistant", "", "m");
            s.messages.push(Message::new("user", text));
            s.derived_title()
        };
        // A lead-in question is not a title — the first task is.
        assert_eq!(
            t("Can you take care of these for me?\n- List recurring paid subscriptions\n- Find renewals"),
            "List recurring paid subscriptions"
        );
        // Polite and plumbing prefixes are shed, including several layers.
        assert_eq!(t("Can you please help me draft the launch email"), "Draft the launch email");
        assert_eq!(t("Use the project-tracker skill to build a tracker"), "Project-tracker skill to build a tracker");
        // Trailing punctuation goes; first letter is capitalised.
        assert_eq!(t("summarise the quarterly numbers."), "Summarise the quarterly numbers");
        // Long input is cut on a word boundary, never mid-word.
        let long = t("Investigate why the nightly batch job keeps failing on the storage cluster and write it up");
        assert!(long.ends_with('…'), "{long}");
        assert!(long.chars().count() <= 57, "{long}");
        assert!(!long.contains("  "));
        // The names actually in this machine's sidebar, which is what prompted the
        // change: raw prompt fragments, cut mid-sentence, unreadable as a column.
        assert_eq!(
            t("Call ask_user right now with THREE questions, each with 3 options that have real descriptions."),
            "Ask_user with THREE questions"
        );
        assert_eq!(t("Summarise the sheet for me. Use ask_user if you need to check something."), "Summarise the sheet");
        // "right now" and "for me" say when and how eagerly, never what.
        assert_eq!(t("Write a status note right now for me"), "Write a status note");
        // A phrase, not a sentence: everything past the first full stop is
        // instructions rather than subject.
        assert_eq!(t("Fix the login bug. It only happens on Safari, and only after a reload."), "Fix the login bug");
        // Both caps mark their cut, so a shortened name never passes as the whole one.
        for long in [
            "Investigate why the nightly batch job keeps failing on the storage cluster",
            "Reconcile the quarterly subscription renewals against the finance spreadsheet",
        ] {
            let out = t(long);
            assert!(out.ends_with('…'), "no ellipsis on a cut name: {out}");
            assert!(out.chars().count() <= 43, "{out}");
        }
        // …and a name that fits is left whole, with no ellipsis to imply otherwise.
        assert_eq!(t("Write me a brief on Q3"), "Write me a brief on Q3");

        // An explicit rename always wins.
        let mut named = Session::new("assistant", "", "m");
        named.title = "My title".into();
        named.messages.push(Message::new("user", "- something else"));
        assert_eq!(named.derived_title(), "My title");
        // No user message yet.
        assert_eq!(Session::new("assistant", "", "m").derived_title(), "New session");
    }
