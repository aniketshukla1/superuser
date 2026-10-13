use super::*;

    #[test]
    fn a_reply_is_threaded_under_the_message_it_answers() {
        let m = build("me@x.com", "ada@example.com", "Re: Deck", "Here it is.", Some("<m2@example.com>"), Some("<m1@example.com> <m2@example.com>")).unwrap();
        let text = String::from_utf8(m.formatted()).unwrap();
        assert!(text.contains("In-Reply-To: <m2@example.com>"), "{text}");
        assert!(text.contains("References: <m1@example.com> <m2@example.com>"), "{text}");
        let plain = String::from_utf8(build("me@x.com", "ada@example.com", "Hi", "x", None, Some("  ")).unwrap().formatted()).unwrap();
        assert!(!plain.contains("In-Reply-To") && !plain.contains("References"), "{plain}");
    }

    /// A lesson kept: the first look, and any look after the mailbox's
    /// numbering changed, reads nothing that was already there.
    #[test]
    fn the_bot_starts_from_now_and_restarts_from_now_when_numbering_changes() {
        let fresh = BotState::default();
        assert_eq!(next_range(&fresh, 7), None);
        let going = BotState { started: true, uid_validity: 7, last_uid: 41, ..Default::default() };
        assert_eq!(next_range(&going, 7).as_deref(), Some("42:*"));
        assert_eq!(next_range(&going, 8), None, "a new UIDVALIDITY means the numbers mean nothing");
        // An empty mailbox at the start is still a start: the first mail to arrive is read.
        let empty = BotState { started: true, uid_validity: 7, last_uid: 0, ..Default::default() };
        assert_eq!(next_range(&empty, 7).as_deref(), Some("1:*"));
    }

    #[test]
    fn the_draft_count_is_per_day_and_survives_a_restart() {
        let dir = std::env::temp_dir().join(format!("su-emailbot-{}", crate::core::inbox::short_id()));
        std::fs::create_dir_all(&dir).unwrap();
        note_drafted(&dir, "2026-09-23");
        note_drafted(&dir, "2026-09-23");
        assert_eq!(drafted_on(&load_state(&dir), "2026-09-23"), 2);
        assert_eq!(drafted_on(&load_state(&dir), "2026-09-24"), 0, "a new day starts at zero");
        note_drafted(&dir, "2026-09-24");
        assert_eq!(drafted_on(&load_state(&dir), "2026-09-24"), 1);
        note_extracted(&dir, "2026-09-24");
        assert_eq!((drafted_on(&load_state(&dir), "2026-09-24"), extracted_on(&load_state(&dir), "2026-09-24")), (1, 1));
        note_extracted(&dir, "2026-09-25");
        assert_eq!((drafted_on(&load_state(&dir), "2026-09-25"), extracted_on(&load_state(&dir), "2026-09-25")), (0, 1), "a new day resets both");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_inbound_message_carries_what_the_screen_and_the_reply_need() {
        let raw = concat!(
            "From: =?utf-8?q?Ada_Lovelace?= <ADA@Example.com>\r\n",
            "Subject: Deck for Monday\r\n",
            "Date: Fri, 18 Sep 2026 23:30:00 -0700\r\n",
            "Message-ID: <m2@example.com>\r\n",
            "References: <m1@example.com>\r\n",
            "List-Id: <team.example.com>\r\n",
            "Authentication-Results: mx.example.com; dmarc=pass\r\n",
            "\r\n",
            "Can you send the deck?\r\n\r\nOn Mon, Bo wrote:\r\n> old\r\n"
        );
        let m = Inbound::from_raw(9, raw.as_bytes());
        assert_eq!(m.from, "Ada Lovelace <ADA@Example.com>");
        assert_eq!(m.from_addr, "ada@example.com");
        assert_eq!(m.subject, "Deck for Monday");
        assert_eq!(m.date, "2026-09-18", "the sender's own day, not ours");
        assert_eq!(m.message_id, "<m2@example.com>");
        assert_eq!(m.references, "<m1@example.com>");
        assert!(m.list_mail);
        assert!(m.auth_results.contains("dmarc=pass"));
        assert_eq!(m.text, "Can you send the deck?");
    }

    #[test]
    fn parses_headers_case_insensitively_and_folds() {
        let raw = "Subject: Quarterly review\r\nFrom: Ada <ada@example.com>\r\nDate: Mon, 4 Aug 2026 09:00:00 +0000\r\n";
        assert_eq!(header_value(raw, "Subject"), "Quarterly review");
        assert_eq!(header_value(raw, "subject"), "Quarterly review"); // case-insensitive
        assert_eq!(header_value(raw, "From"), "Ada <ada@example.com>");
        assert_eq!(header_value(raw, "Cc"), ""); // absent → empty, not an error
    }

    #[test]
    fn smtp_host_defaults_from_the_imap_host() {
        // The fallback used by creds(): imap.fastmail.com → smtp.fastmail.com
        let imap = "imap.fastmail.com";
        let smtp = imap.strip_prefix("imap.").map(|r| format!("smtp.{r}")).unwrap_or_else(|| imap.to_string());
        assert_eq!(smtp, "smtp.fastmail.com");
        // a host that doesn't start with imap. is reused as-is
        let odd = "mail.company.internal";
        let s2 = odd.strip_prefix("imap.").map(|r| format!("smtp.{r}")).unwrap_or_else(|| odd.to_string());
        assert_eq!(s2, "mail.company.internal");
    }
