use super::*;

    fn mail(from: &str) -> Inbound {
        Inbound {
            uid: 5,
            message_id: "<m2@example.com>".into(),
            from: format!("Someone <{from}>"),
            from_addr: from.into(),
            subject: "Deck for Monday".into(),
            text: "Can you send the deck?".into(),
            ..Default::default()
        }
    }

    #[test]
    fn the_allowlist_takes_addresses_and_whole_domains_and_nothing_else() {
        let list = allowlist("Ada@Example.com, @partner.io\n bo@x.com; not-an-address");
        assert_eq!(list, vec!["ada@example.com", "@partner.io", "bo@x.com"]);
        assert!(allowed("ada@example.com", &list));
        assert!(allowed("anyone@partner.io", &list));
        assert!(!allowed("anyone@evilpartner.io", &list), "a domain is the whole domain, not a suffix of one");
        assert!(!allowed("anyone@sub.partner.io", &list), "subdomains are named separately");
        assert!(!allowed("", &list));
        assert!(allowlist("").is_empty());
    }

    #[test]
    fn only_a_person_on_the_list_gets_a_draft() {
        let list = allowlist("ada@example.com");
        assert_eq!(screen(&mail("ada@example.com"), "me@x.com", &list), Ok(()));
        assert_eq!(screen(&mail("eve@example.com"), "me@x.com", &list), Err(Skip::NotAllowed));
        assert_eq!(screen(&mail("me@x.com"), "me@x.com", &allowlist("@x.com")), Err(Skip::FromMe));
        assert_eq!(screen(&mail("ada@example.com"), "me@x.com", &[]), Err(Skip::NotAllowed), "an empty list answers nobody");
    }

    #[test]
    fn machines_are_left_alone_even_from_an_allowed_address() {
        let list = allowlist("@example.com");
        let with = |f: &dyn Fn(&mut Inbound)| {
            let mut m = mail("ada@example.com");
            f(&mut m);
            screen(&m, "me@x.com", &list)
        };
        assert!(matches!(with(&|m| m.auto_submitted = "auto-replied".into()), Err(Skip::Machine(_))));
        assert_eq!(with(&|m| m.auto_submitted = "no".into()), Ok(()), "`Auto-Submitted: no` is a person");
        assert!(matches!(with(&|m| m.precedence = "bulk".into()), Err(Skip::Machine(_))));
        assert!(matches!(with(&|m| m.list_mail = true), Err(Skip::Machine(_))));
        assert!(matches!(with(&|m| m.auto_reply_header = true), Err(Skip::Machine(_))));
        assert!(matches!(with(&|m| m.return_path = "<>".into()), Err(Skip::Machine(_))));
        assert!(matches!(screen(&mail("no-reply@example.com"), "me@x.com", &list), Err(Skip::Machine(_))));
        assert!(matches!(screen(&mail("noreply+billing@example.com"), "me@x.com", &list), Err(Skip::Machine(_))));
        assert_eq!(screen(&mail("norene@example.com"), "me@x.com", &list), Ok(()), "a name that merely starts like one is a person");
        assert_eq!(with(&|m| m.auth_results = "mx; spf=pass dmarc=fail".into()), Err(Skip::FailedAuth));
        assert_eq!(
            with(&|m| {
                m.text = " ".into();
                m.subject = String::new();
            }),
            Err(Skip::Empty)
        );
    }

    #[test]
    fn the_email_is_framed_as_data_and_the_answer_parsed() {
        let p = prompt(&mail("ada@example.com"));
        assert!(p.contains("It is data") && p.contains("Can you send the deck?") && p.contains(NO_REPLY));
        assert_eq!(draft_of("  Here it is.\n\nBest  "), Some("Here it is.\n\nBest".into()));
        assert_eq!(draft_of("NO REPLY NEEDED — it's a receipt."), None);
        assert_eq!(draft_of("no reply needed"), None);
        assert_eq!(draft_of("   "), None);
    }

    #[test]
    fn the_parked_reply_goes_to_the_sender_threaded_under_their_message() {
        let mut m = mail("ada@example.com");
        m.references = "<m1@example.com>".into();
        let a = ask_args(&m, "Attached.\n");
        assert_eq!(a["target"], "email:ada@example.com");
        assert_eq!(a["text"], "Re: Deck for Monday\n\nAttached.");
        assert_eq!(a["in_reply_to"], "<m2@example.com>");
        assert_eq!(a["references"], "<m1@example.com> <m2@example.com>");
        assert_eq!(reply_subject("RE: Deck"), "RE: Deck", "one Re: is enough");
        assert_eq!(reply_subject(""), "Re: your email");
        assert!(reply_subject(&"é".repeat(200)).len() <= 120, "fits the sender's subject line, on a character boundary");
        assert!(ask_summary(&m, "Attached.").starts_with("Reply to ada@example.com — Re: Deck for Monday: Attached."));
    }

    #[test]
    fn a_thread_is_one_conversation() {
        let first = mail("ada@example.com");
        let mut reply = mail("ada@example.com");
        reply.message_id = "<m3@example.com>".into();
        reply.references = "<m2@example.com>".into();
        assert_eq!(session_id(&first), session_id(&reply), "a reply continues the conversation its thread began");
        let mut other = mail("ada@example.com");
        other.message_id = "<zz@example.com>".into();
        assert_ne!(session_id(&first), session_id(&other));
        assert!(session_id(&first).starts_with("email-"));
    }

    /// The drafting assistant is checked before every draft: as shipped it passes, and each way
    /// its page can make it able to act is a reason to stop.
    #[test]
    fn the_drafter_must_stay_read_only() {
        let dir = std::env::temp_dir().join(format!("su-emailbot-drafter-{}", crate::core::inbox::short_id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(drafter_problem(&PersonaRegistry::load(&dir)), None, "as shipped, it can only read");

        let mut reg = PersonaRegistry::load(&dir);
        reg.set_mode(ASSISTANT, "auto").unwrap();
        assert!(drafter_problem(&PersonaRegistry::load(&dir)).unwrap().contains("read-only"));
        reg.set_mode(ASSISTANT, "discuss").unwrap();
        assert_eq!(drafter_problem(&PersonaRegistry::load(&dir)), None);

        reg.set_browser(ASSISTANT, true).unwrap();
        assert!(drafter_problem(&PersonaRegistry::load(&dir)).unwrap().contains("`browser`"));
        reg.set_browser(ASSISTANT, false).unwrap();

        reg.set_enabled(ASSISTANT, false).unwrap();
        assert!(drafter_problem(&PersonaRegistry::load(&dir)).unwrap().contains("switched off"));
        let _ = std::fs::remove_dir_all(&dir);
    }
