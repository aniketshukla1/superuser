use super::*;
    use serde_json::json;

    fn scratch() -> std::path::PathBuf {
        static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let d = std::env::temp_dir().join(format!("su-gateway-{}-{}-{n}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn facts(session: &str) -> Facts {
        Facts { persona: "assistant".into(), session: session.into(), initiator: Initiator::from_session(session) }
    }

    /// The gateway's contract as `run_tool_with` uses it, with a fake tool.
    fn run(dir: &Path, f: &Facts, name: &str, args: Value, tool: impl FnOnce() -> String) -> String {
        let subject = subject_for(name, &args, None, false);
        match before(dir, f, subject) {
            Err(refusal) => refusal,
            Ok(ticket) => {
                let out = tool();
                after(dir, f, &ticket, &out, false);
                out
            }
        }
    }

    fn deny(dir: &Path, name: &str, when: &str, mode: boundaries::Mode) {
        boundaries::save(
            dir,
            &boundaries::Boundaries {
                mode,
                deny: vec![boundaries::Rule { name: name.into(), when: when.into(), ..Default::default() }],
                ..Default::default()
            },
        )
        .unwrap();
    }

    #[test]
    fn a_refused_action_never_runs_and_is_written_down_first() {
        let dir = scratch();
        deny(&dir, "No env files", r#"file.name == ".env""#, boundaries::Mode::Enforce);
        let mut ran = false;
        let out = run(&dir, &facts("s1"), "write_file", json!({"path": "app/.env", "content": "KEY=1"}), || {
            ran = true;
            "wrote app/.env".into()
        });
        assert!(!ran, "the tool must not run");
        assert!(out.starts_with("Refused."), "{out}");
        assert!(out.contains("No env files") && out.contains("don't try another way"), "{out}");
        let rows = audit::query(&dir, &audit::Filter { session: Some("s1".into()), event: Some("action".into()), ..Default::default() });
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].event_name(), "action.refused");
        let d = rows[0].decision.as_ref().unwrap();
        assert_eq!((d.rule_name.as_str(), d.carried_out), ("No env files", false));
        // What was typed is not in the trail — only how much.
        let text = std::fs::read_to_string(dir.join(audit::FILE)).unwrap();
        assert!(!text.contains("KEY=1"), "{text}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn record_mode_writes_the_would_be_refusal_and_runs_it() {
        let dir = scratch();
        deny(&dir, "No env files", r#"file.name == ".env""#, boundaries::Mode::Record);
        let mut ran = false;
        let out = run(&dir, &facts("s1"), "write_file", json!({"path": ".env", "content": "x"}), || {
            ran = true;
            "wrote .env (1 bytes)".into()
        });
        assert!(ran);
        assert!(out.starts_with("wrote"));
        let rows = audit::query(&dir, &audit::Filter { event: Some("action".into()), ..Default::default() });
        assert_eq!(rows[0].event_name(), "action.allowed");
        let d = rows[0].decision.as_ref().unwrap();
        assert!(!d.allowed && d.carried_out && d.mode == "record");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn every_outcome_row_follows_its_decision_row() {
        // The mechanical check: for every action that has an
        // outcome row, the decision row for it comes first.
        let dir = scratch();
        let f = facts("s9");
        run(&dir, &f, "run_shell", json!({"command": "false"}), || "error: exit code 1".into());
        run(&dir, &f, "send_message", json!({"target": "slack:#ops", "text": "hi"}), || {
            "Parked for approval (inbox id x). The message was NOT sent".into()
        });
        run(&dir, &f, "run_shell", json!({"command": "rm -rf build"}), || {
            "Denied by the user — the command was NOT run".into()
        });
        run(&dir, &f, "write_file", json!({"path": "a.md", "content": "x"}), || "wrote a.md (1 bytes)".into());
        run(&dir, &f, "recall_memory", json!({"query": "x"}), || "no relevant memories found".into());

        let mut rows = audit::query(&dir, &audit::Filter { session: Some("s9".into()), ..Default::default() });
        rows.reverse(); // oldest first
        let events: Vec<&str> = rows.iter().map(|r| r.event_name()).collect();
        assert_eq!(
            events,
            vec![
                "action.allowed", "action.failed",
                "action.allowed", "action.parked",
                "action.allowed", "action.declined",
                "action.allowed",
            ],
            "a recall is not recorded when allowed; everything that reaches past the conversation is"
        );
        for (i, r) in rows.iter().enumerate() {
            if r.decision.is_none() {
                let prior = rows[..i].iter().rev().find(|p| p.decision.is_some()).expect("an outcome row with no decision before it");
                assert_eq!(prior.subject.as_ref().unwrap().tool, r.subject.as_ref().unwrap().tool);
            }
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_stop_is_its_own_outcome() {
        let dir = scratch();
        let f = facts("s2");
        let ticket = before(&dir, &f, subject_for("run_shell", &json!({"command": "sleep 100"}), None, false)).unwrap();
        after(&dir, &f, &ticket, "", true);
        let rows = audit::recent(&dir, 1);
        assert_eq!(rows[0].event_name(), "action.stopped");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_rule_sees_what_started_the_run() {
        let dir = scratch();
        deny(&dir, "", r#"initiator.kind == "schedule" && intent == "send""#, boundaries::Mode::Enforce);
        let send = || subject_for("send_message", &json!({"target": "slack:#ops", "text": "hi"}), None, false);
        assert!(before(&dir, &facts("__run__run-1"), send()).is_err());
        assert!(before(&dir, &facts("s-person"), send()).is_ok());
        let rows = audit::query(&dir, &audit::Filter { nobody_watching: true, ..Default::default() });
        assert_eq!(rows.len(), 1, "the scheduled refusal is the unwatched row");
        assert_eq!(rows[0].initiator.as_ref().unwrap().kind, "schedule");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn intent_follows_what_the_action_does() {
        assert_eq!(intent_of("edit_file", &json!({}), None), "write_file");
        assert_eq!(intent_of("table", &json!({"spec": {}}), None), "show");
        assert_eq!(intent_of("table", &json!({"path": "out.csv"}), None), "write_file");
        assert_eq!(intent_of("process_send", &json!({}), None), "run_command");
        assert_eq!(intent_of("browser_open", &json!({}), None), "navigate");
        // The assistant's own browser: what a keystroke does decides what it is.
        assert_eq!(intent_of("browser_navigate", &json!({}), None), "navigate");
        assert_eq!(intent_of("browser_click", &json!({}), None), "activate");
        assert_eq!(intent_of("browser_type", &json!({"text": "shoes"}), None), "type");
        assert_eq!(intent_of("browser_type", &json!({"text": "shoes", "submit": true}), None), "activate");
        assert_eq!(intent_of("browser_key", &json!({"key": "Enter"}), None), "activate");
        assert_eq!(intent_of("browser_key", &json!({"key": "Control+A"}), None), "type");
        assert_eq!(intent_of("browser_scroll", &json!({}), None), "read");
        let read = McpCall { server: "linear".into(), tool: "list_issues".into(), effect: "read".into() };
        let write = McpCall { effect: "write".into(), ..read.clone() };
        assert_eq!(intent_of("linear_list_issues", &json!({}), Some(&read)), "read_tool");
        assert_eq!(intent_of("linear_create_issue", &json!({}), Some(&write)), "write_tool");
        // Every catalogue tool has an intent — a new tool with none would be the one
        // no rule could name.
        for (_, name) in crate::core::agent::every_tool() {
            if UNGOVERNED.contains(&name.as_str()) {
                continue;
            }
            assert!(!intent_of(&name, &json!({}), None).is_empty(), "{name} has no intent");
        }
    }

    #[test]
    fn the_subject_never_carries_content() {
        let s = subject_for("send_message", &json!({"target": "email:ceo@example.com", "text": "the password is hunter2"}), None, true);
        assert_eq!(s.chars, Some(23));
        assert!(serde_json::to_string(&s).unwrap().find("hunter2").is_none());
        let s = subject_for("web_fetch", &json!({"url": "https://example.com/a?token=abc"}), None, false);
        assert_eq!((s.page.as_str(), s.target.as_str()), ("https://example.com/a", "example.com"));
        let s = subject_for("web_search", &json!({"query": "my medical results"}), None, false);
        assert!(serde_json::to_string(&s).unwrap().find("medical").is_none());
        // What was typed into a page is its length and nothing else.
        let s = subject_for("browser_type", &json!({"ref": "e4", "snapshot_id": 2, "text": "hunter2", "submit": true}), None, true);
        assert_eq!((s.element_ref.as_str(), s.chars, s.key.as_str()), ("e4", Some(7), "Enter"));
        assert!(serde_json::to_string(&s).unwrap().find("hunter2").is_none());
        // No browser for this persona: nothing to read the element from, and nothing invented.
        let s = with_browser_element(subject_for("browser_click", &json!({"ref": "e1", "snapshot_id": 1}), None, false), "gw-nobody");
        assert!(s.element_name.is_empty() && s.page.is_empty() && s.target.is_empty());
        // Everything else passes through untouched.
        let fetched = subject_for("web_fetch", &json!({"url": "https://example.com/a"}), None, false);
        assert_eq!(serde_json::to_string(&with_browser_element(fetched.clone(), "gw-nobody")).unwrap(), serde_json::to_string(&fetched).unwrap());
    }

    #[test]
    fn a_refusal_reads_as_refused_not_as_an_error() {
        assert_eq!(status_of("Refused. Writing .env was stopped"), "refused");
        assert_eq!(status_of("error: no such file"), "error");
        assert_eq!(status_of("wrote a.md"), "ok");
        assert_eq!(outcome_of("Refused. x"), None, "a refusal is its own decision row, not an outcome");
    }
