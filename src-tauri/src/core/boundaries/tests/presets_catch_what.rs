use super::*;

    pub(super) fn scratch() -> PathBuf {
        static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let d = std::env::temp_dir().join(format!("su-boundaries-{}-{}-{n}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    pub(super) fn write(intent: &str, file: &str) -> Subject {
        Subject { tool: "write_file".into(), intent: intent.into(), file: file.into(), risk: "write_local".into(), ..Default::default() }
    }

    pub(super) fn person() -> Initiator {
        Initiator::new("person", "")
    }

    pub(super) fn decide(b: &Boundaries, s: &Subject) -> Decision {
        compile(b.clone(), None).decide(&context(s, "assistant", "s1", &person()))
    }

    #[test]
    pub(super) fn the_default_refuses_nothing() {
        let d = decide(&Boundaries::default(), &write("write_file", "notes.md"));
        assert!(d.allowed && d.carried_out);
        assert_eq!(d.source, "allow");
    }

    #[test]
    pub(super) fn a_deny_beats_a_broader_allow() {
        let b = Boundaries {
            deny: vec![Rule { name: "No env files".into(), when: r#"file.name == ".env""#.into(), ..Default::default() }],
            ..Default::default()
        };
        let d = decide(&b, &write("write_file", "app/.env"));
        assert!(!d.allowed && !d.carried_out);
        assert_eq!((d.source.as_str(), d.rule_name.as_str()), ("deny", "No env files"));
        assert!(decide(&b, &write("write_file", "app/.envrc")).allowed);
    }

    #[test]
    pub(super) fn record_mode_decides_and_lets_it_through() {
        let b = Boundaries {
            mode: Mode::Record,
            deny: vec![Rule { when: r#"intent == "write_file""#.into(), ..Default::default() }],
            ..Default::default()
        };
        let d = decide(&b, &write("write_file", "a.md"));
        assert!(!d.allowed, "the rules still say no");
        assert!(d.carried_out, "and it runs anyway");
        assert_eq!(d.mode, "record");
    }

    #[test]
    pub(super) fn a_broken_deny_denies_and_says_it_is_broken() {
        // Saved by hand, past the page's check.
        let b = Boundaries {
            deny: vec![Rule { name: "Typo".into(), when: r#"file.nmae == ".env""#.into(), ..Default::default() }],
            ..Default::default()
        };
        let d = decide(&b, &write("write_file", "a.md"));
        assert!(!d.allowed && !d.carried_out);
        assert!(d.reason.contains("can't be evaluated"), "{d:?}");
        let words = refusal(&write("write_file", "a.md"), &d, None);
        assert!(words.starts_with("Refused."), "{words}");
        assert!(words.contains("can't be read"), "{words}");

        // A rule answering text is broken the same way.
        let b = Boundaries { deny: vec![Rule { when: r#""Submit order""#.into(), ..Default::default() }], ..Default::default() };
        assert!(!decide(&b, &write("write_file", "a.md")).allowed);
    }

    #[test]
    pub(super) fn a_broken_allow_permits_nothing_and_an_empty_allow_refuses_everything() {
        let b = Boundaries { allow: vec!["intnt == \"read\"".into()], ..Default::default() };
        let d = decide(&b, &write("read", ""));
        assert!(!d.allowed);
        assert_eq!(d.source, "default");

        let b = Boundaries { allow: vec![], ..Default::default() };
        let d = decide(&b, &write("read_file", "a.md"));
        assert!(!d.allowed);
        assert!(refusal(&write("read_file", "a.md"), &d, None).contains("allow list"));
    }

    #[test]
    pub(super) fn every_field_is_bound_so_a_rule_about_commands_is_false_about_files() {
        // The failure this prevents: a deny rule naming `command`, evaluated for a file
        // write, throwing on an unbound field — and a throwing deny refuses.
        let b = Boundaries { deny: vec![Rule { when: r#"contains(command, "rm")"#.into(), ..Default::default() }], ..Default::default() };
        assert!(decide(&b, &write("write_file", "a.md")).allowed);
        for (field, _) in FIELDS {
            let expr = format!("{field} == {field}");
            let p = rules::parse(&expr).unwrap();
            let ctx = context(&Subject::default(), "", "", &person());
            assert_eq!(p.test(&ctx), Ok(true), "{field} is documented but not bound");
        }
    }

    #[test]
    pub(super) fn every_preset_is_a_valid_rule_with_a_cost() {
        for p in PRESETS {
            assert_eq!(check_rule(p.when), None, "{}", p.id);
            assert!(p.cost.len() > 40, "{} needs to say what it costs", p.id);
        }
    }

    #[test]
    pub(super) fn the_presets_catch_what_their_names_say_and_not_their_neighbours() {
        let preset = |id: &str| Boundaries {
            deny: vec![Rule { when: PRESETS.iter().find(|p| p.id == id).unwrap().when.into(), ..Default::default() }],
            ..Default::default()
        };
        let cmd = |c: &str| Subject { tool: "run_shell".into(), intent: "run_command".into(), command: c.into(), ..Default::default() };
        let nav = |u: &str| Subject { tool: "web_fetch".into(), intent: "navigate".into(), page: u.into(), ..Default::default() };

        let del = preset("no-deleting-commands");
        assert!(!decide(&del, &cmd("rm -rf build")).allowed);
        assert!(!decide(&del, &cmd("cd x && rm a.txt")).allowed);
        assert!(!decide(&del, &cmd("git reset --hard HEAD~1")).allowed);
        assert!(decide(&del, &cmd("npm run format")).allowed, "`rm` inside a word is not a delete");
        assert!(decide(&del, &cmd("ls -la")).allowed);

        let social = preset("no-social-media");
        assert!(!decide(&social, &nav("https://www.x.com/home")).allowed);
        assert!(!decide(&social, &nav("https://x.com/")).allowed);
        assert!(decide(&social, &nav("https://dropbox.com/s/1")).allowed, "x.com is not a substring match");

        let secrets = preset("no-secret-files");
        assert!(!decide(&secrets, &write("read_file", "service/.env")).allowed);
        assert!(!decide(&secrets, &write("read_file", "/Users/me/.ssh/id_ed25519")).allowed);
        assert!(!decide(&secrets, &write("write_file", "certs/server.PEM")).allowed);
        assert!(!decide(&secrets, &cmd("cat .env")).allowed);
        assert!(decide(&secrets, &write("read_file", "src/environment.ts")).allowed);

        let outside = preset("no-acting-after-reading-outside");
        let mut send = Subject { tool: "send_message".into(), intent: "send".into(), target: "slack:#ops".into(), ..Default::default() };
        assert!(decide(&outside, &send).allowed, "a clean run may send");
        send.tainted = true;
        assert!(!decide(&outside, &send).allowed, "a run that read the web may not");

        let submit = preset("never-submit-in-browser");
        let press = |label: &str| Subject {
            tool: "browser_click".into(), intent: "activate".into(), element_role: "button".into(), element_name: label.into(),
            page: "https://shop.example/cart".into(), target: "shop.example".into(), ..Default::default()
        };
        assert!(!decide(&submit, &press("Place order")).allowed);
        assert!(!decide(&submit, &press("SUBMIT")).allowed, "labels match whatever their case");
        assert!(!decide(&submit, &Subject { tool: "browser_key".into(), intent: "activate".into(), key: "Enter".into(), ..Default::default() }).allowed);
        assert!(!decide(&submit, &Subject { tool: "browser_type".into(), intent: "activate".into(), key: "Enter".into(), chars: Some(5), ..Default::default() }).allowed);
        assert!(decide(&submit, &Subject { tool: "browser_type".into(), intent: "type".into(), element_name: "Search".into(), chars: Some(5), ..Default::default() }).allowed, "typing without Enter is not submitting");
        assert!(decide(&submit, &press("Next page")).allowed);
        assert!(decide(&submit, &press("Continue")).allowed, "the stated cost: a submit labelled Continue gets through");

        let unattended = preset("unattended-only-reads");
        let ctx = |kind: &str| context(&write("write_file", "report.md"), "assistant", "s", &Initiator::new(kind, ""));
        let loaded = compile(unattended.clone(), None);
        assert!(!loaded.decide(&ctx("schedule")).allowed);
        assert!(loaded.decide(&ctx("person")).allowed);
    }

    #[test]
    pub(super) fn checking_a_rule_names_the_field_that_does_not_exist() {
        let msg = check_rule(r#"elemnt.name == "Submit""#).unwrap();
        assert!(msg.contains("`elemnt.name`") && msg.contains("`element.name`"), "{msg}");
        let msg = check_rule(r#"intent = "send""#).unwrap();
        assert!(msg.contains("=="), "{msg}");
        let msg = check_rule(r#""Submit order""#).unwrap();
        assert!(msg.contains("true or false"), "{msg}");
        assert_eq!(check_rule(r#"intent == "send" && tainted"#), None);
        // A group on its own is a real value.
        assert_eq!(check_rule(r#"size(page) > 0"#), None);
    }
