use super::*;

    fn scratch() -> PathBuf {
        static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let seq = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let d = std::env::temp_dir().join(format!("su-prov-{}-{}-{}", std::process::id(), t, seq));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn src(s: &str) -> Source {
        Source { source: s.into(), kind: "file".into(), quote: None, at: 0.0 }
    }

    /// The whole point, end to end: a real receipt, read, written into a cell, and
    /// still traceable back to the line it came from.
    ///
    /// Ignored by default because it needs a PDF on disk. Run with:
    ///   SU_PDF=… cargo test --no-default-features -- --ignored a_number_can_be_traced
    #[test]
    #[ignore]
    fn a_number_can_be_traced_back_to_the_receipt_it_came_from() {
        let Ok(pdf) = std::env::var("SU_PDF") else { return };
        // 1. Read the receipt, as the agent would.
        let text = crate::core::files::extract(&pdf).expect("extract").text.unwrap_or_default();
        let amount = text
            .lines()
            .find_map(|l| l.trim().strip_prefix("Amount:"))
            .map(|a| a.trim().split_whitespace().next().unwrap_or("").to_string())
            .expect("the receipt should state an amount");
        assert_eq!(amount, "12.00");

        // 2. Write it into a workbook, citing where it came from.
        let d = scratch();
        let wb = d.join("tracker.xlsx");
        crate::core::sheets::create(&wb, &["Subscriptions".into()]).unwrap();
        crate::core::sheets::write_cells(
            &wb,
            Some("Subscriptions"),
            &[
                crate::core::sheets::CellEdit { cell: "A2".into(), value: "Notion".into(), result: None },
                crate::core::sheets::CellEdit { cell: "B2".into(), value: amount.clone(), result: None },
            ],
        )
        .unwrap();
        record(
            &wb,
            "Subscriptions",
            &[(
                "B2".into(),
                Source {
                    source: pdf.clone(),
                    kind: "file".into(),
                    quote: Some("Amount: 12.00 USD".into()),
                    at: 0.0,
                },
            )],
        )
        .unwrap();

        // 3. The number is in the sheet, and it still says where it came from.
        let rows = crate::core::sheets::read_rows(&wb, Some("Subscriptions"), 5, 5).unwrap();
        assert_eq!(rows[1], vec!["Notion", "12"]);
        let account = describe(&load(&wb), 10);
        assert!(account.contains("B2"), "{account}");
        assert!(account.contains(&pdf), "the cell no longer points at the receipt: {account}");
        assert!(account.contains("Amount: 12.00 USD"), "the quote is what makes it checkable: {account}");
    }

    #[test]
    fn the_record_lives_beside_the_workbook_not_inside_it() {
        // A column of URLs makes the spreadsheet worse to read and worse to print,
        // and the user editing a cell should not have to maintain a citation in the
        // next column.
        let p = Path::new("/tmp/ws/budget.xlsx");
        assert_eq!(ledger_path(p), Path::new("/tmp/ws/budget.xlsx.sources.json"));
    }

    #[test]
    fn a_citation_can_always_be_looked_up_whatever_case_it_was_written_in() {
        assert_eq!(key("Subscriptions", "b7"), key("subscriptions", "B7"));
    }

    #[test]
    fn a_source_survives_the_round_trip() {
        let d = scratch();
        let wb = d.join("t.xlsx");
        record(&wb, "Subscriptions", &[("B7".into(), src("receipts/notion-aug.pdf"))]).unwrap();
        let l = load(&wb);
        assert_eq!(l.cells[&key("Subscriptions", "B7")].source, "receipts/notion-aug.pdf");
    }

    #[test]
    fn recording_more_sources_keeps_the_ones_already_there() {
        let d = scratch();
        let wb = d.join("t.xlsx");
        record(&wb, "S", &[("A1".into(), src("one.pdf"))]).unwrap();
        record(&wb, "S", &[("A2".into(), src("two.pdf"))]).unwrap();
        assert_eq!(load(&wb).cells.len(), 2);
    }

    #[test]
    fn rewriting_a_cell_replaces_its_source_rather_than_keeping_the_old_claim() {
        // The old receipt is no longer where that number came from.
        let d = scratch();
        let wb = d.join("t.xlsx");
        record(&wb, "S", &[("A1".into(), src("old.pdf"))]).unwrap();
        record(&wb, "S", &[("A1".into(), src("new.pdf"))]).unwrap();
        let l = load(&wb);
        assert_eq!(l.cells.len(), 1);
        assert_eq!(l.cells[&key("S", "A1")].source, "new.pdf");
    }

    #[test]
    fn a_citation_for_a_deleted_cell_is_dropped() {
        // Otherwise it is a claim about a value that no longer exists.
        let d = scratch();
        let wb = d.join("t.xlsx");
        record(&wb, "S", &[("A1".into(), src("a.pdf")), ("A2".into(), src("b.pdf"))]).unwrap();
        let dropped = prune(&wb, &[key("S", "A1")]).unwrap();
        assert_eq!(dropped, 1);
        assert_eq!(load(&wb).cells.len(), 1);
    }

    #[test]
    fn a_corrupt_record_reads_as_empty_rather_than_breaking_the_workbook() {
        let d = scratch();
        let wb = d.join("t.xlsx");
        std::fs::write(ledger_path(&wb), b"{ not json").unwrap();
        assert_eq!(load(&wb), Ledger::default());
    }

    #[test]
    fn the_account_names_the_cell_and_its_source() {
        let mut l = Ledger::default();
        l.cells.insert(
            key("S", "B7"),
            Source { source: "receipts/notion.pdf".into(), kind: "file".into(), quote: Some("Amount: 12.00 USD".into()), at: 0.0 },
        );
        let out = describe(&l, 10);
        assert!(out.contains("B7") && out.contains("receipts/notion.pdf"));
        assert!(out.contains("Amount: 12.00 USD"), "the quote is what makes it checkable: {out}");
    }

    #[test]
    fn an_account_of_nothing_says_so() {
        assert!(describe(&Ledger::default(), 5).contains("No sources"));
    }

    #[test]
    fn a_long_account_is_capped_and_says_how_much_it_left_out() {
        let mut l = Ledger::default();
        for i in 1..=30 {
            l.cells.insert(key("S", &format!("A{i}")), src("x.pdf"));
        }
        let out = describe(&l, 5);
        assert!(out.contains("and 25 more"), "{out}");
    }
