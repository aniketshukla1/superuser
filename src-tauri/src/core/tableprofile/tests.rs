use super::*;

    fn rows(v: &[&[&str]]) -> Vec<Vec<String>> {
        v.iter().map(|r| r.iter().map(|s| s.to_string()).collect()).collect()
    }

    #[test]
    fn it_says_what_each_column_is() {
        let p = profile(&rows(&[
            &["month", "revenue", "region", "customer"],
            &["2026-01-05", "1,200", "EU", "Acme"],
            &["2026-02-05", "1400", "EU", "Belex"],
            &["2026-03-05", "12,900", "US", "Corvid"],
            &["2026-04-05", "1750", "US", "Dunhill"],
        ]));
        assert_eq!(p.rows, 4);
        let by = |n: &str| p.columns.iter().find(|c| c.name == n).unwrap().clone();

        assert_eq!(by("month").kind, ColumnType::Date, "a date column is the x-axis");
        assert_eq!(by("revenue").kind, ColumnType::Number);
        // A thousands separator is formatting, not text. The extremes are BOTH comma-bearing
        // on purpose: with a comma value merely present in the middle of the range, min and
        // max come out the same whether or not commas are stripped, and the check proves
        // nothing — which is exactly what the first version of this test did.
        assert_eq!(by("revenue").min, Some(1200.0));
        assert_eq!(by("revenue").max, Some(12900.0));
        assert_eq!(by("region").kind, ColumnType::Category, "few distinct values group");
        assert_eq!(by("region").top.first().unwrap().0, "EU");
        // All distinct: a bar chart of this is one bar per row, and the agent needs to know.
        assert_eq!(by("customer").kind, ColumnType::Text);
    }

    #[test]
    fn a_year_column_is_a_number() {
        let p = profile(&rows(&[&["year"], &["2024"], &["2025"], &["2026"]]));
        assert_eq!(p.columns[0].kind, ColumnType::Number);
    }

    #[test]
    fn blanks_are_not_values() {
        let p = profile(&rows(&[&["a", "b"], &["1", ""], &["2", "  "], &["", ""]]));
        assert_eq!(p.columns[0].filled, 2, "an empty cell is not a value");
        assert_eq!(p.columns[1].kind, ColumnType::Empty);
        assert_eq!(p.rows, 3, "…but the row count is rows, not filled cells");
    }

    #[test]
    fn a_ragged_row_does_not_lose_its_column() {
        // Real exports do this: a trailing column present on some rows only.
        let p = profile(&rows(&[&["a", "b", "c"], &["1", "x"], &["2", "y", "extra"]]));
        assert_eq!(p.columns.len(), 3);
        assert_eq!(p.columns[2].filled, 1);
    }

    #[test]
    fn an_empty_table_says_so_rather_than_panicking() {
        assert_eq!(profile(&[]).rows, 0);
        assert_eq!(profile(&rows(&[&["only", "a", "header"]])).rows, 0);
    }
