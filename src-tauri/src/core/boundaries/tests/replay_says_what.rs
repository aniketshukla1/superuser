use super::*;

    #[test]
    pub(super) fn nothing_invalid_is_ever_saved() {
        let dir = scratch();
        let bad = Boundaries { deny: vec![Rule { when: "intent ==".into(), ..Default::default() }], ..Default::default() };
        let problems = save(&dir, &bad).unwrap_err();
        assert_eq!(problems[0].list, "deny");
        assert!(!path(&dir).exists());

        let good = Boundaries { deny: vec![Rule { name: "No sending".into(), when: r#"intent == "send""#.into(), ..Default::default() }], ..Default::default() };
        save(&dir, &good).unwrap();
        assert_eq!(load(&dir).boundaries, good);
        assert!(crate::core::audit::recent(&dir, 5).iter().any(|e| e.event_name() == "policy.changed"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    pub(super) fn an_unreadable_file_refuses_everything_instead_of_lifting_every_rule() {
        let dir = scratch();
        std::fs::write(path(&dir), "{ this is not json").unwrap();
        let loaded = load(&dir);
        assert!(loaded.problem.is_some());
        let d = loaded.decide(&context(&write("read_file", "a.md"), "assistant", "s", &person()));
        assert!(!d.allowed && !d.carried_out);
        let words = refusal(&write("read_file", "a.md"), &d, loaded.problem.as_deref());
        assert!(words.contains("boundaries file can't be read"), "{words}");

        // Fixed on disk, it reads again — the cache follows the file.
        std::thread::sleep(std::time::Duration::from_millis(10));
        std::fs::write(path(&dir), r#"{"mode":"enforce","deny":[],"allow":["true"]}"#).unwrap();
        assert!(load(&dir).problem.is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    pub(super) fn a_dry_run_policy_file_reads_as_it_is() {
        let text = r#"{"mode":"dry-run","deny":["intent == \"navigate\" && contains(page.host, \"facebook.com\")"],"allow":["true"]}"#;
        let b: Boundaries = serde_json::from_str(text).unwrap();
        assert_eq!(b.mode, Mode::Record);
        assert_eq!(b.deny[0].when, r#"intent == "navigate" && contains(page.host, "facebook.com")"#);
        assert!(check(&b).is_empty());
    }

    #[test]
    pub(super) fn replay_says_what_a_candidate_would_have_changed() {
        let row = |intent: &str, file: &str, allowed: bool| {
            let mut e = Entry::event(if allowed { "action.allowed" } else { "action.refused" });
            e.session = "s1".into();
            e.persona = "assistant".into();
            e.subject = Some(write(intent, file));
            e.decision = Some(Decision { allowed, mode: "enforce".into(), source: if allowed { "allow" } else { "deny" }.into(), carried_out: allowed, ..Default::default() });
            e
        };
        let mut failed = Entry::event("action.failed");
        failed.subject = Some(write("write_file", ".env"));
        failed.decision = Some(Decision { allowed: true, carried_out: true, ..Default::default() });
        let rows = vec![
            row("write_file", "app/.env", true),
            row("write_file", "notes.md", true),
            row("read_file", "b/.env", false),
            failed,
        ];
        let candidate = Boundaries {
            deny: vec![Rule { name: "No env files".into(), when: r#"file.name == ".env""#.into(), ..Default::default() }],
            ..Default::default()
        };
        let r = replay(&candidate, &rows);
        assert_eq!(r.scanned, 3, "the failure row is about an action already counted");
        assert_eq!((r.would_refuse, r.would_allow, r.unchanged), (1, 0, 2));
        assert_eq!(r.changes[0].what, "writing app/.env");
        assert_eq!(r.changes[0].rule_name, "No env files");

        // And the other direction: removing the rule that refused the read.
        let r = replay(&Boundaries::default(), &rows);
        assert_eq!((r.would_refuse, r.would_allow), (0, 1));
        assert_eq!(r.changes[0].would, "allowed");
    }

    #[test]
    pub(super) fn file_parts_and_hosts_come_out_the_way_rules_expect() {
        assert_eq!(file_parts("a/b/.env"), (".env".into(), "".into()));
        assert_eq!(file_parts("a/b/Server.PEM"), ("Server.PEM".into(), "pem".into()));
        assert_eq!(file_parts("archive.tar.gz"), ("archive.tar.gz".into(), "gz".into()));
        assert_eq!(file_parts(""), ("".into(), "".into()));
        assert_eq!(host_of("https://User:pw@Shop.Example.COM.:8443/x?y"), "shop.example.com");
        assert_eq!(host_of("http://[::1]:3000/"), "::1");
        assert_eq!(host_of("not a url"), "");
    }
