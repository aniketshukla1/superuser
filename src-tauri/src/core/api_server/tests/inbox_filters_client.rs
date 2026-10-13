use super::*;

#[test]
    pub(super) fn the_inbox_filters_the_client_has_always_sent_are_actually_applied() {
        // `getInbox(sessionId, "pending")` has sent both since it was written and the
        // route read neither, so a chat session was handed every item in the inbox —
        // including a question an unattended automation parked hours earlier and that
        // had already been denied. It rendered as a live card, and clicking an option
        // posted a resolution for an already-resolved item: nothing happened, twice
        // over, with no error anywhere.
        let ask = |sid: &str, status: &str| {
            let mut a = crate::core::inbox::Ask::new(sid, None, "ask_user", "user", "q?", json!({}));
            a.status = status.into();
            a
        };
        let mine = ask("chat-1", "pending");
        let mine_done = ask("chat-1", "denied");
        let theirs = ask("__run__run-f56", "pending");

        let q = |sid: &str, state: &str| InboxQuery { session_id: sid.into(), state: state.into() };

        // One conversation's open questions: exactly one of these three.
        assert!(inbox_matches(&mine, &q("chat-1", "pending")));
        assert!(!inbox_matches(&mine_done, &q("chat-1", "pending")), "resolved is not pending");
        assert!(!inbox_matches(&theirs, &q("chat-1", "pending")), "another session's is not mine");

        // `resolved` is the API's word for every terminal status the record uses.
        assert!(inbox_matches(&mine_done, &q("", "resolved")));
        assert!(!inbox_matches(&mine, &q("", "resolved")));

        // And the Inbox view, which asks for everything, still gets everything —
        // filtering by default would empty a page nobody asked to narrow.
        let all = InboxQuery::default();
        for a in [&mine, &mine_done, &theirs] {
            assert!(inbox_matches(a, &all));
        }
    }

    /// The forwarder's trapdoor, held open.
    ///
    /// Every one of these is a kind the GUI already knows how to render. The test is
    /// not that the mapping is clever — it is that a kind can never go missing from
    /// it, because a missing kind fails silently on the far side of a channel.
    #[test]
    pub(super) fn every_run_event_the_gui_renders_survives_the_forwarder() {
        let cases = [
            (json!({"type": "tool", "name": "read_file", "arguments": {"path": "a.md"}, "text": "read a.md"}), "tool_proposed"),
            (json!({"type": "tool_done", "name": "read_file", "status": "ok", "text": "…"}), "tool_finished"),
            (json!({"type": "action", "text": "clicked"}), "tool_proposed"),
            (json!({"type": "question", "questions": [{"question": "Which?"}]}), "question_requested"),
            (json!({"type": "permission", "name": "run_shell", "arguments": {"command": "ls"}}), "permission_required"),
            (json!({"type": "text", "text": "hello"}), "assistant_delta"),
            (json!({"type": "thinking", "text": "hmm"}), "reasoning_delta"),
            (json!({"type": "stopped"}), "interrupted"),
            (json!({"type": "usage", "text": "1200"}), "usage"),
            (json!({"type": "computer_frame", "persona": "assistant", "frame": "f-1-0001", "url": "https://example.com/", "title": "Example"}), "computer_frame"),
            (json!({"type": "computer_help", "persona": "assistant", "reason": "Sign in", "url": "https://example.com/"}), "computer_help"),
            (json!({"type": "computer_secret", "persona": "assistant", "label": "Password"}), "computer_secret"),
        ];
        for (ev, expected) in cases {
            let out = gui_frame(&ev).unwrap_or_else(|| panic!("{ev} was dropped on the way to the GUI"));
            assert_eq!(out["type"], expected, "for {ev}");
        }
    }

    /// The Skills page's switch: `{enabled}` switches a skill everywhere,
    /// an edit it can't make is refused rather than answered `ok`, an unknown skill is an error,
    /// and the route the page calls — `PATCH /v1/skills/:name` — exists. It didn't: the page's
    /// switch has sent that request since the fork, to a path that only answered DELETE.
    #[test]
    pub(super) fn the_skills_switch_switches_and_refuses_what_it_cant_do() {
        let dir = std::env::temp_dir().join(format!("su-skillswitch-{}", crate::core::inbox::short_id()));
        let sd = dir.join("skills").join("greeter");
        std::fs::create_dir_all(&sd).unwrap();
        std::fs::write(sd.join("SKILL.md"), "---\nname: greeter\ndescription: says hi\n---\nSay hello.").unwrap();
        let on = |d: &std::path::Path| crate::core::skills::SkillStore::load(d).is_on("greeter");

        assert_eq!(switch_skill(&dir, "greeter", &json!({"enabled": false}))["ok"], true);
        assert!(!on(&dir));
        assert_eq!(switch_skill(&dir, "greeter", &json!({"enabled": true}))["ok"], true);
        assert!(on(&dir));
        assert_eq!(switch_skill(&dir, "greeter", &json!({"description": "new words"}))["ok"], false);
        assert_eq!(switch_skill(&dir, "greeter", &json!({}))["ok"], false);
        assert_eq!(switch_skill(&dir, "no-such-skill", &json!({"enabled": false}))["ok"], false);
        assert!(on(&dir), "nothing a refusal said changed the switch");

        let src = &crate::test_support::module_source("core/api_server");
        assert!(src.contains(".route(\"/v1/skills/:name\", axum::routing::delete(skill_uninstall).patch(skill_update))"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A parked approval carries what the Inbox card reads, so a drafted email shows as the
    /// message it is — not as a JSON string of its arguments.
    #[test]
    pub(super) fn a_parked_approval_carries_what_the_inbox_card_reads() {
        let ask = crate::core::inbox::Ask::new(
            "email-abc",
            None,
            "send_message",
            "email:ada@example.com",
            "Reply to ada@example.com — Re: Deck: Here it is.",
            json!({ "target": "email:ada@example.com", "text": "Re: Deck\n\nHere it is.", "in_reply_to": "<m2@x>" }),
        );
        let item = inbox_item(&ask);
        assert_eq!(item["kind"], "approval");
        assert_eq!(item["data"]["tool"], "send_message");
        assert_eq!(item["data"]["arguments"]["text"], "Re: Deck\n\nHere it is.");
        assert_eq!(item["state"], "pending");
    }

    /// The Expense Manager's routes: the settings patch takes its own two fields, the status
    /// counts the month from the ledger, and the weekly routine is added once, for the Expense
    /// Manager, on Mondays.
    #[test]
    pub(super) fn the_expense_routes_write_only_their_own_settings_and_add_the_routine_once() {
        assert_eq!(
            expenses_patch(&json!({ "expense_intake": true, "expense_policy": "Meals up to 50 USD.", "email_inbound": true })),
            json!({ "expense_intake": true, "expense_policy": "Meals up to 50 USD." })
        );
        let dir = std::env::temp_dir().join(format!("su-expenses-api-{}", crate::core::inbox::short_id()));
        std::fs::create_dir_all(&dir).unwrap();
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        crate::core::expenses::append(
            &dir,
            crate::core::expenses::Expense { id: "r1".into(), date: today, merchant: "Cafe".into(), amount: "4.00".into(), message_id: "<a@b>".into(), ..Default::default() },
        )
        .unwrap();
        let status = expenses_status(&crate::core::settings::Settings::default(), &dir);
        assert_eq!((status["receipts"].clone(), status["this_month"].clone(), status["routine"].clone()), (json!(1), json!(1), json!(false)));

        assert_eq!(add_expense_routine(&dir)["created"], true);
        assert_eq!(add_expense_routine(&dir)["created"], false, "added once");
        let tasks = crate::core::automation::AutomationStore::load(&dir).list();
        assert_eq!(tasks.len(), 1);
        assert_eq!((tasks[0].agent.as_str(), tasks[0].schedule.cron.as_deref()), (crate::core::expenses::ASSISTANT, Some("0 9 * * 1")));
        assert_eq!(expenses_status(&crate::core::settings::Settings::default(), &dir)["routine"], true);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The Chief of Staff's routes: the patch takes the priorities and nothing else, the status
    /// says what feeds the digest, and the weekday routine is added once, for the Chief of Staff.
    #[test]
    pub(super) fn the_chief_of_staff_routes_write_only_the_priorities_and_add_the_routine_once() {
        assert_eq!(chief_patch(&json!({ "cos_priorities": "Ship billing", "expense_intake": true })), json!({ "cos_priorities": "Ship billing" }));
        assert_eq!(chief_patch(&json!({ "cos_priorities": 3 })), json!({}));
        let dir = std::env::temp_dir().join(format!("su-chief-api-{}", crate::core::inbox::short_id()));
        std::fs::create_dir_all(&dir).unwrap();
        let memory = crate::core::memory::MemoryEngine::new(dir.join("vault"));
        memory
            .save(&crate::core::memory::MemoryInput {
                title: "PR #42 needs review".into(),
                category: "GitHub".into(),
                summary: "Waiting on you".into(),
                body: "Waiting on you".into(),
                origin: crate::core::memory::Origin::External,
                ..Default::default()
            })
            .unwrap();
        let status = chief_status(&crate::core::settings::Settings::default(), &dir, &memory);
        assert_eq!(status["sources"], json!([{ "source": "GitHub", "items": 1 }]));
        assert_eq!(status["routine"], false);
        let cos = |d: &std::path::Path| add_routine(d, crate::core::chief_of_staff::ASSISTANT, crate::core::chief_of_staff::ROUTINE_TITLE, crate::core::chief_of_staff::ROUTINE_CRON, crate::core::chief_of_staff::ROUTINE_INSTRUCTIONS);
        assert_eq!(cos(&dir)["created"], true);
        assert_eq!(cos(&dir)["created"], false, "added once");
        let task = &crate::core::automation::AutomationStore::load(&dir).list()[0];
        assert_eq!((task.agent.as_str(), task.schedule.cron.as_deref()), ("chief", Some("0 8 * * 1-5")));
        let _ = std::fs::remove_dir_all(&dir);
    }
