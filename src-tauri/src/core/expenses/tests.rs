use super::*;

    fn receipt(subject: &str, text: &str) -> Inbound {
        Inbound {
            uid: 1,
            message_id: "<r1@uber.com>".into(),
            from: "Uber Receipts <noreply@uber.com>".into(),
            from_addr: "noreply@uber.com".into(),
            subject: subject.into(),
            date: "2026-09-18".into(),
            text: text.into(),
            ..Default::default()
        }
    }

    fn scratch() -> PathBuf {
        let d = std::env::temp_dir().join(format!("su-expenses-{}", crate::core::inbox::short_id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn a_receipt_is_told_from_a_newsletter_before_any_model_is_asked() {
        assert!(looks_like_receipt(&receipt("Your Thursday evening trip with Uber", "Total ₹452.30. Thanks for riding. Your receipt")));
        assert!(looks_like_receipt(&receipt("Invoice #1042", "Amount due: USD 1,299.00")));
        assert!(!looks_like_receipt(&receipt("Our spring sale!", "Save 20% — prices from $19.99")), "an amount is not enough");
        assert!(!looks_like_receipt(&receipt("Your receipt", "We'll email it when the order ships.")), "nor is the word");
    }

    #[test]
    fn an_extraction_is_checked_field_by_field() {
        let m = receipt("Your trip", "…");
        let got = parse_extraction(
            "Sure! {\"merchant\": \"Uber\", \"date\": \"2026-09-18\", \"amount\": \"452.3\", \"currency\": \"inr\", \"category\": \"transport\", \"note\": \"ride to the office\"}",
            &m,
            "2026-09-20",
        )
        .unwrap()
        .unwrap();
        assert_eq!((got.merchant.as_str(), got.amount.as_str(), got.currency.as_str(), got.category.as_str()), ("Uber", "452.30", "INR", "Transport"));
        assert_eq!(got.source, "noreply@uber.com");
        assert_eq!(got.message_id, "<r1@uber.com>");
        assert_eq!(parse_extraction("{\"not_a_receipt\": true}", &m, "2026-09-20"), Ok(None));
        assert!(parse_extraction("{\"merchant\": \"X\", \"amount\": \"free\"}", &m, "2026-09-20").is_err());
        assert!(parse_extraction("{\"merchant\": \"X\", \"amount\": \"-5.00\"}", &m, "2026-09-20").unwrap_err().contains("refund"));
        assert!(parse_extraction("{\"merchant\": \"\", \"amount\": \"5\"}", &m, "2026-09-20").is_err());
        assert!(parse_extraction("no json here", &m, "2026-09-20").is_err());
        // What it can't read it leaves empty, rather than guess.
        let odd = parse_extraction("{\"merchant\": \"Shop\", \"amount\": 9.5, \"currency\": \"dollars\", \"category\": \"Snacks\", \"date\": \"yesterday\"}", &m, "2026-09-20")
            .unwrap()
            .unwrap();
        assert_eq!((odd.amount.as_str(), odd.currency.as_str(), odd.category.as_str(), odd.date.as_str()), ("9.50", "", "", "2026-09-18"));
    }

    #[test]
    fn amounts_read_in_either_convention() {
        assert_eq!(parse_cents("12.34"), Some(1234));
        assert_eq!(parse_cents("$1,234.56"), Some(123456));
        assert_eq!(parse_cents("1.234,56 €"), Some(123456));
        assert_eq!(parse_cents("12,5"), Some(1250));
        assert_eq!(parse_cents("1200"), Some(120000));
        assert_eq!(parse_cents("nothing"), None);
        assert_eq!(money(123456789), "1,234,567.89");
    }

    #[test]
    fn the_ledger_round_trips_and_a_spreadsheet_cannot_run_a_cell() {
        let dir = scratch();
        let e = Expense {
            id: "e1".into(),
            date: "2026-09-18".into(),
            merchant: "=HYPERLINK(\"http://evil\")".into(),
            amount: "12.00".into(),
            currency: "USD".into(),
            category: "Meals".into(),
            note: "lunch, with \"Bo\"\nand Ada".into(),
            source: "a@b.com".into(),
            subject: "Receipt".into(),
            message_id: "<m1@x>".into(),
            added: "2026-09-18T10:00:00+00:00".into(),
        };
        assert_eq!(append(&dir, e.clone()), Ok(Added::Logged));
        let raw = std::fs::read_to_string(ledger_path(&dir)).unwrap();
        assert!(raw.starts_with("id,date,merchant"));
        assert!(raw.contains("\"'=HYPERLINK(\"\"http://evil\"\")\""), "the formula is written as text: {raw}");
        let back = load(&dir);
        assert_eq!(back.len(), 1);
        assert_eq!(back[0].note, e.note);
        assert_eq!(back[0].merchant, "'=HYPERLINK(\"http://evil\")");
        assert_eq!(append(&dir, e.clone()), Ok(Added::AlreadyLogged), "the same email is never logged twice");
        let again = Expense { id: "e2".into(), message_id: "<m2@x>".into(), merchant: "'=HYPERLINK(\"http://evil\")".into(), ..e };
        append(&dir, again).unwrap();
        assert!(load(&dir)[1].note.contains("possible duplicate of e1"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn line(id: &str, date: &str, merchant: &str, amount: &str, currency: &str, category: &str) -> Expense {
        Expense {
            id: id.into(),
            date: date.into(),
            merchant: merchant.into(),
            amount: amount.into(),
            currency: currency.into(),
            category: category.into(),
            source: "r@shop.com".into(),
            subject: format!("{merchant} receipt"),
            ..Default::default()
        }
    }

    /// The job's rule: totals that reconcile back to the source.
    #[test]
    fn the_totals_reconcile_to_the_receipts_they_list() {
        let all = vec![
            line("a", "2026-09-14", "Uber", "10.50", "USD", "Transport"),
            line("b", "2026-09-15", "Lyft", "4.50", "USD", "Transport"),
            line("c", "2026-09-16", "Cafe", "8.00", "USD", ""),
            line("d", "2026-09-16", "Dosa", "250.00", "INR", "Meals"),
            line("e", "2026-09-30", "Hotel", "999.00", "USD", "Lodging"),
        ];
        let p = Period { from: chrono::NaiveDate::from_ymd_opt(2026, 9, 14).unwrap(), to: chrono::NaiveDate::from_ymd_opt(2026, 9, 20).unwrap() };
        let s = summarize(&all, &p, "Meals up to 50 USD a day.\n\nNo alcohol.");
        assert!(s.contains("— 4 receipts."), "{s}");
        assert!(s.contains("- Transport: 15.00 USD (#1, #2)"), "{s}");
        assert!(s.contains("- Uncategorized: 8.00 USD (#3)"), "{s}");
        assert!(s.contains("- Meals: 250.00 INR (#4)"), "{s}");
        assert!(s.contains("Total: 250.00 INR; 23.00 USD"), "{s}");
        assert!(!s.contains("Hotel"), "outside the period");
        assert!(s.contains("Missing a category: #3"), "{s}");
        assert!(s.contains("P1 Meals up to 50 USD a day.") && s.contains("P2 No alcohol."), "policy lines are numbered for citing: {s}");
        assert!(summarize(&all, &p, "").contains("No policy is set"));
    }

    #[test]
    fn periods_are_named_not_computed_by_the_model() {
        let today = chrono::NaiveDate::from_ymd_opt(2026, 9, 24).unwrap(); // a Thursday
        let d = |s: &str| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap();
        assert_eq!(period("this_week", None, None, today).unwrap(), Period { from: d("2026-09-21"), to: today });
        assert_eq!(period("last_week", None, None, today).unwrap(), Period { from: d("2026-09-14"), to: d("2026-09-20") });
        assert_eq!(period("this_month", None, None, today).unwrap(), Period { from: d("2026-09-01"), to: today });
        assert_eq!(period("last_month", None, None, today).unwrap(), Period { from: d("2026-08-01"), to: d("2026-08-31") });
        assert_eq!(period("", Some("2026-09-01"), Some("2026-09-10"), today).unwrap(), Period { from: d("2026-09-01"), to: d("2026-09-10") });
        assert!(period("fortnight", None, None, today).is_err());
        assert!(period("", Some("2026-09-10"), Some("2026-09-01"), today).is_err());
    }
