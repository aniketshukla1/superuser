use super::*;

    #[test]
    pub(super) fn a_formula_is_stored_as_a_formula_not_as_its_text() {
        let d = scratch();
        let p = d.join("t.xlsx");
        create(&p, &["S".into()]).unwrap();
        write_cells(&p, None, &[edit("B2", "12"), edit("B3", "8"), edit("B4", "=SUM(B2:B3)")]).unwrap();
        let book = umya_spreadsheet::reader::xlsx::read(&p).unwrap();
        let ws = book.get_sheet_by_name("S").unwrap();
        assert_eq!(ws.get_cell("B4").unwrap().get_formula(), "SUM(B2:B3)");
    }

    #[test]
    pub(super) fn writing_to_a_sheet_that_is_not_there_says_so() {
        // Rather than writing to whichever sheet happened to be first, which the
        // user would find much later and in the wrong place.
        let d = scratch();
        let p = d.join("t.xlsx");
        create(&p, &["Subscriptions".into(), "Lookup Lists".into()]).unwrap();
        let err = write_cells(&p, Some("Sheet3"), &[edit("A1", "x")]).unwrap_err();
        assert!(err.contains("Sheet3") && err.contains("Subscriptions"), "got: {err}");
    }

    #[test]
    pub(super) fn a_bad_reference_is_refused_before_anything_is_saved() {
        let d = scratch();
        let p = d.join("t.xlsx");
        create(&p, &["S".into()]).unwrap();
        write_cells(&p, None, &[edit("A1", "keep me")]).unwrap();
        let err = write_cells(&p, None, &[edit("A2", "fine"), edit("nonsense", "x")]).unwrap_err();
        assert!(err.contains("nonsense"));
        // The whole batch is refused, so the file never ends up half-applied.
        let rows = read_rows(&p, None, 10, 10).unwrap();
        assert_eq!(rows[0], vec!["keep me"]);
        assert!(rows.get(1).map(|r| r.is_empty()).unwrap_or(true), "a refused batch still wrote something");
    }

    #[test]
    pub(super) fn a_named_sheet_is_matched_whatever_its_case() {
        let d = scratch();
        let p = d.join("t.xlsx");
        create(&p, &["Subscriptions".into(), "Lookup Lists".into()]).unwrap();
        write_cells(&p, Some("lookup lists"), &[edit("A1", "Monthly")]).unwrap();
        let rows = read_rows(&p, Some("Lookup Lists"), 5, 5).unwrap();
        assert_eq!(rows[0], vec!["Monthly"]);
    }

    #[test]
    pub(super) fn reading_is_capped_so_one_workbook_cannot_eat_a_turn() {
        let d = scratch();
        let p = d.join("big.xlsx");
        create(&p, &["S".into()]).unwrap();
        let edits: Vec<CellEdit> = (1..=50).map(|r| edit(&format!("A{r}"), &format!("row {r}"))).collect();
        write_cells(&p, None, &edits).unwrap();
        let rows = read_rows(&p, None, 10, 5).unwrap();
        assert_eq!(rows.len(), 10, "the cap was not applied");
    }

    #[test]
    pub(super) fn trailing_empty_cells_are_not_padded_into_the_models_context() {
        let d = scratch();
        let p = d.join("t.xlsx");
        create(&p, &["S".into()]).unwrap();
        write_cells(&p, None, &[edit("A1", "one"), edit("E1", "five")]).unwrap();
        let rows = read_rows(&p, None, 3, 20).unwrap();
        assert_eq!(rows[0].len(), 5, "row should stop at the last non-empty cell");
    }
