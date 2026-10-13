use super::*;

    fn scratch() -> std::path::PathBuf {
        let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let d = std::env::temp_dir().join(format!("bba-dataload-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn a_csv_comes_back_as_rows_with_its_numbers_still_numbers() {
        let d = scratch();
        std::fs::write(d.join("q3.csv"), "quarter,revenue,note\nQ1,\"1,200\",\"steady, flat\"\nQ2,$1400,up\nQ3,,missing\n").unwrap();
        let rows = read(&d, "q3.csv").unwrap();
        assert_eq!(rows.columns, vec!["quarter", "revenue", "note"]);
        assert_eq!(rows.rows.len(), 3);
        // A thousands separator and a currency symbol are still a number to a person.
        assert_eq!(rows.rows[0]["revenue"], serde_json::json!(1200.0));
        assert_eq!(rows.rows[1]["revenue"], serde_json::json!(1400.0));
        // A quoted comma is one cell, not two.
        assert_eq!(rows.rows[0]["note"], serde_json::json!("steady, flat"));
        // An empty cell is nothing, not zero — a chart that draws it as zero is a lie.
        assert_eq!(rows.rows[2]["revenue"], Value::Null);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn json_rows_read_from_an_array_or_from_the_array_inside_an_object() {
        let d = scratch();
        std::fs::write(d.join("a.json"), r#"[{"day":"Mon","count":3},{"day":"Tue","count":5,"note":"busy"}]"#).unwrap();
        let rows = read(&d, "a.json").unwrap();
        // Alphabetical, because that is the order the JSON parser hands keys back in —
        // a chart over JSON names its fields rather than trusting this.
        assert_eq!(rows.columns, vec!["count", "day", "note"]);
        assert_eq!(rows.rows.len(), 2);
        std::fs::write(d.join("b.json"), r#"{"generated":"today","data":[{"day":"Mon","count":3}]}"#).unwrap();
        assert_eq!(read(&d, "b.json").unwrap().rows.len(), 1);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn every_refusal_is_a_sentence_that_says_what_to_do() {
        let d = scratch();
        std::fs::write(d.join("empty.csv"), "a,b\n").unwrap();
        std::fs::write(d.join("bad.json"), "{not json").unwrap();
        assert!(read(&d, "empty.csv").unwrap_err().contains("no rows"));
        assert!(read(&d, "bad.json").unwrap_err().contains("isn't valid JSON"));
        assert!(read(&d, "missing.csv").unwrap_err().contains("couldn't read"));
        assert!(read(&d, "notes.md").unwrap_err().contains("isn't something this can read"));
        assert!(read(&d, "../secrets.csv").unwrap_err().contains("isn't a path in your folder"));
        assert!(read(&d, "the figures in the user's message").unwrap_err().contains("no extension"));
        // What a caller checks before trying at all.
        assert!(is_readable("q3.csv") && is_readable("data.JSON") && !is_readable("notes.md"));
        assert!(!is_readable("the figures the user gave me"));
        let _ = std::fs::remove_dir_all(&d);
    }
