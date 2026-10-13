use super::*;

    // An `@` mention's reply is another assistant's. The next run reads it marked as
    // theirs and without their tool traffic (those were their tools, not its), and a
    // client still sees whose it was.
    #[test]
    pub(super) fn another_assistants_reply_is_marked_in_the_history_and_kept_for_the_client() {
        let dir = scratch();
        let store = SessionStore::new(&dir);
        let s = Session::new("chief", "", "m");
        store.save(&s);
        store.append(&s.session_id, Message::new("user", "@expenses what did we spend?"));
        let wire = crate::core::transcript::TurnWire {
            shape: crate::core::transcript::WireShape::Claude,
            model: "m".into(),
            messages: vec![serde_json::json!({ "role": "assistant", "content": [{ "type": "tool_use", "name": "ledger_query" }] })],
        };
        let mut theirs = Message::new("assistant", "£412 this week.");
        theirs.persona = Some("expenses".into());
        theirs.wire = Some(wire.clone());
        store.append(&s.session_id, theirs);
        let mut own = Message::new("assistant", "Noted.");
        own.wire = Some(wire);
        store.append(&s.session_id, own);

        let back = store.get(&s.session_id).unwrap();
        let h = back.history_wired();
        assert_eq!(h[1].1, "[Answered by @expenses.]\n\n£412 this week.");
        assert!(h[1].2.is_none(), "their tool traffic stays out of the next run");
        assert_eq!(h[2].1, "Noted.", "the conversation's own replies are unmarked");
        assert!(h[2].2.is_some(), "and keep theirs");
        assert_eq!(back.messages[1].without_wire().persona.as_deref(), Some("expenses"));
        assert!(!serde_json::to_string(&back.messages[2]).unwrap().contains("persona"), "absent unless set");
        let _ = fs::remove_dir_all(&dir);
    }
