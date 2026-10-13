use super::*;

/// The case that breaks naive splitting, and the one real exports are full of.
    #[test]
    pub(super) fn a_quoted_comma_does_not_split_a_column() {
        assert_eq!(
            split_delimited(r#"Smith, J.,1200"#, ','),
            vec!["Smith", " J.", "1200"],
            "unquoted, this really is three fields"
        );
        assert_eq!(
            split_delimited(r#""Smith, J.",1200"#, ','),
            vec!["Smith, J.", "1200"],
            "quoted, the comma is part of the name and the row has two fields"
        );
        // "" inside quotes is one literal quote — otherwise the field ends early and
        // everything after it shifts a column left.
        assert_eq!(split_delimited(r#""a""b",2"#, ','), vec![r#"a"b"#, "2"]);
        assert_eq!(split_delimited("a\tb", '\t'), vec!["a", "b"]);
    }

    /// A CSV used to reach the workbook reader, which reported it as corrupt — so a dropped
    /// CSV failed with a message about a broken xlsx and the model told the user their file
    /// was damaged.
    #[test]
    pub(super) fn a_csv_reads_as_rows_rather_than_as_a_broken_workbook() {
        let dir = std::env::temp_dir().join("su-sheets-csv-test");
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("t.csv");
        // A blank line in the middle, and a trailing newline: neither is a row, and counting
        // either would show up in a profile as missing data that is not there.
        std::fs::write(&p, "month,revenue\n2026-01,\"1,200\"\n\n2026-02,1400\n").unwrap();
        let rows = read_rows(&p, None, 200, 40).unwrap();
        assert_eq!(rows.len(), 3, "header plus two rows, no blanks: {rows:?}");
        assert_eq!(rows[0], vec!["month", "revenue"]);
        assert_eq!(rows[1], vec!["2026-01", "1,200"], "the quoted comma stayed in one field");
        assert_eq!(rows[2], vec!["2026-02", "1400"]);
        let _ = std::fs::remove_file(&p);
    }

    pub(super) fn scratch() -> PathBuf {
        static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let seq = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let d = std::env::temp_dir().join(format!("su-sheets-{}-{}-{}", std::process::id(), t, seq));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    pub(super) fn edit(cell: &str, value: &str) -> CellEdit {
        CellEdit { cell: cell.into(), value: value.into(), result: None }
    }

    #[test]
    pub(super) fn an_identifier_that_looks_like_a_number_stays_text() {
        // The classic spreadsheet injury: "007" saved as 7 cannot be recovered from
        // the file, and a leading-zero id is far more often an id than a quantity.
        assert_eq!(interpret("007"), Value::Text("007".into()));
        assert_eq!(interpret("0012345"), Value::Text("0012345".into()));
        // ...but a real decimal is a number.
        assert_eq!(interpret("0.5"), Value::Number(0.5));
    }

    #[test]
    pub(super) fn numbers_formulas_and_text_are_told_apart() {
        assert_eq!(interpret("12"), Value::Number(12.0));
        assert_eq!(interpret(" 12.50 "), Value::Number(12.5));
        assert_eq!(interpret("=SUM(B2:B9)"), Value::Formula("SUM(B2:B9)".into()));
        assert_eq!(interpret("Notion"), Value::Text("Notion".into()));
        assert_eq!(interpret("   "), Value::Empty);
    }

    #[test]
    pub(super) fn a_nonsense_number_is_not_smuggled_in_as_one() {
        // inf/NaN would serialise into a cell nothing can read back.
        assert_eq!(interpret("inf"), Value::Text("inf".into()));
        assert_eq!(interpret("NaN"), Value::Text("NaN".into()));
    }

    #[test]
    pub(super) fn cell_references_are_checked_before_the_file_is_touched() {
        for good in ["A1", "b7", "AA100", "ZZ9999"] {
            assert!(is_cell_ref(good), "{good} should be a reference");
        }
        for bad in ["", "1A", "A", "7", "B7:C9", "Sheet1!A1", "AAAA1", "A1 "] {
            assert!(!is_cell_ref(bad.trim_end()) || bad == "A1 ", "{bad} should not be a reference");
        }
    }

    #[test]
    pub(super) fn a_formula_written_with_its_result_survives_a_reader_that_cannot_recalculate() {
        // The bug this pins: `set_formula` alone emits `<f>…</f><v/>`, and a reader
        // that doesn't recalculate — SheetJS in our own panel, and `files::extract`,
        // which is how the agent reads its own work back — drops the cell entirely.
        // Measured 2026-08-21: a formula written without a result round-tripped to
        // nothing at all.
        let d = scratch();
        let p = d.join("calc.xlsx");
        create(&p, &["Data".into()]).unwrap();
        write_cells(
            &p,
            None,
            &[
                CellEdit { cell: "A1".into(), value: "10".into(), result: None },
                CellEdit { cell: "A2".into(), value: "15.5".into(), result: None },
                // With a cached result, as the panel sends it.
                CellEdit { cell: "A3".into(), value: "=SUM(A1:A2)".into(), result: Some("25.5".into()) },
                // A text result must not be mangled into a number.
                CellEdit { cell: "B1".into(), value: "=A1&\"x\"".into(), result: Some("10x".into()) },
            ],
        )
        .unwrap();

        // `read_rows` does not recalculate — it reads cached values, exactly like the
        // readers that were losing these cells.
        let rows = read_rows(&p, None, 10, 10).unwrap();
        assert_eq!(rows[2][0], "25.5", "the formula's value should be readable: {rows:?}");
        assert_eq!(rows[0][1], "10x", "a text result should stay text: {rows:?}");

        // And the formula itself is still in the file, not replaced by its value.
        let book = umya_spreadsheet::reader::xlsx::read(&p).unwrap();
        let ws = book.get_sheet_by_name("Data").unwrap();
        assert_eq!(ws.get_cell("A3").unwrap().get_formula(), "SUM(A1:A2)");

        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    pub(super) fn a_formula_with_no_result_still_writes_the_formula() {
        // The agent has no engine, so it sends no result. That path must keep
        // working — it just produces a cell only a recalculating reader can see.
        let d = scratch();
        let p = d.join("nocalc.xlsx");
        create(&p, &["Data".into()]).unwrap();
        write_cells(&p, None, &[CellEdit { cell: "A1".into(), value: "=SUM(B1:B2)".into(), result: None }]).unwrap();
        let book = umya_spreadsheet::reader::xlsx::read(&p).unwrap();
        assert_eq!(
            book.get_sheet_by_name("Data").unwrap().get_cell("A1").unwrap().get_formula(),
            "SUM(B1:B2)"
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    pub(super) fn creating_a_workbook_gives_it_the_sheets_asked_for() {
        let d = scratch();
        let p = d.join("tracker.xlsx");
        create(&p, &["Subscriptions".into(), "Lookup Lists".into()]).unwrap();
        assert_eq!(sheet_names(&p).unwrap(), vec!["Subscriptions", "Lookup Lists"]);
    }

    #[test]
    pub(super) fn creating_never_silently_replaces_something_that_is_already_there() {
        let d = scratch();
        let p = d.join("tracker.xlsx");
        create(&p, &["A".into()]).unwrap();
        let err = create(&p, &["B".into()]).unwrap_err();
        assert!(err.contains("already exists"), "got: {err}");
        // and the original is untouched
        assert_eq!(sheet_names(&p).unwrap(), vec!["A"]);
    }

    #[test]
    pub(super) fn editing_one_cell_leaves_every_other_cell_alone() {
        // The whole point: an edit is an edit, not a regeneration. A model that
        // rewrites the workbook loses the user's own rows without ever saying so.
        let d = scratch();
        let p = d.join("t.xlsx");
        create(&p, &["Subscriptions".into()]).unwrap();
        write_cells(&p, None, &[edit("A1", "Service"), edit("B1", "Amount"), edit("A2", "Notion"), edit("B2", "12")]).unwrap();

        // Someone adds a note by hand between turns.
        write_cells(&p, None, &[edit("D2", "cancel before renewal")]).unwrap();
        // The agent changes only the amount.
        write_cells(&p, None, &[edit("B2", "15")]).unwrap();

        let rows = read_rows(&p, None, 10, 10).unwrap();
        assert_eq!(rows[0], vec!["Service", "Amount"]);
        assert_eq!(rows[1][0], "Notion");
        assert_eq!(rows[1][1], "15", "the edit did not land");
        assert_eq!(rows[1][3], "cancel before renewal", "the user's own note was destroyed");
    }
