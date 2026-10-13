use super::*;

    pub(super) fn scratch() -> PathBuf {
        static SCRATCH_N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let seq = SCRATCH_N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let dir = std::env::temp_dir().join(format!("bba-agentexec-{}-{}-{}", std::process::id(), n, seq));
        let _ = std::fs::create_dir_all(&dir);
        dir
    }

    pub(super) fn chart_spec() -> serde_json::Value {
        json!({
            "type": "bar",
            "title": "Revenue by region",
            "note": "North carried the quarter.",
            "x": {"field": "region"},
            "y": {"field": "revenue"},
            "data": [{"region": "North", "revenue": 120}, {"region": "South", "revenue": 90}]
        })
    }

    #[test]
    pub(super) fn a_card_says_what_it_shows_and_writes_nothing() {
        let root = scratch();
        let todo = Mutex::new(Vec::new());
        let spec = json!({
            "title": "Runway",
            "tone": "warn",
            "facts": [{"label": "Cash", "value": "£1.24m"},
                      {"label": "Runs out", "value": "March 2027", "emphasis": true}]
        });
        let out = dispatch_local(&root, &todo, "card", &json!({"spec": spec, "source": "ledger.xlsx"})).unwrap();
        assert!(out.contains("shown in the conversation"), "got: {out}");
        // A card is a way of SAYING something, not a deliverable. Nothing to write.
        assert!(std::fs::read_dir(&root).unwrap().next().is_none(), "a card must not touch disk");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    pub(super) fn a_card_of_half_facts_or_a_whole_table_is_refused() {
        let root = scratch();
        let todo = Mutex::new(Vec::new());
        let bad = [
            json!({"spec": {"facts": []}, "source": "s"}),
            // A value with no label is a number with no meaning — the failure a card exists to avoid.
            json!({"spec": {"facts": [{"label": "", "value": "12"}]}, "source": "s"}),
            json!({"spec": {"facts": [{"label": "Cash"}]}, "source": "s"}),
        ];
        for b in bad {
            let out = dispatch_local(&root, &todo, "card", &b).unwrap();
            assert!(out.starts_with("error:"), "got: {out}");
        }
        // Past a handful it is a table, and saying so points at the right tool.
        let many: Vec<_> = (0..9).map(|i| json!({"label": format!("f{i}"), "value": i})).collect();
        let out = dispatch_local(&root, &todo, "card", &json!({"spec": {"facts": many}, "source": "s"})).unwrap();
        assert!(out.contains("use `table`"), "got: {out}");
        let _ = std::fs::remove_dir_all(&root);
    }

    pub(super) fn table_spec() -> serde_json::Value {
        json!({
            "title": "Revenue by region",
            "columns": [{"field": "region", "label": "Region"},
                        {"field": "revenue", "label": "Revenue", "kind": "number"}],
            "rows": [{"region": "North", "revenue": 1200}, {"region": "South", "revenue": 90}]
        })
    }

    #[test]
    pub(super) fn a_table_is_shown_without_leaving_a_file_behind() {
        let root = scratch();
        let todo = Mutex::new(Vec::new());
        let out = dispatch_local(&root, &todo, "table", &json!({"spec": table_spec(), "source": "q.csv"})).unwrap();
        assert!(out.contains("shown in the conversation"), "got: {out}");
        assert!(out.contains("not saved"), "got: {out}");
        assert!(std::fs::read_dir(&root).unwrap().next().is_none(), "nothing should be written");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    pub(super) fn a_saved_table_is_csv_anyone_can_open() {
        // CSV rather than our own JSON: a saved table is for taking somewhere else,
        // and nothing outside this app reads our shape. (A chart is the opposite —
        // its file IS how the panel redraws it.)
        let root = scratch();
        let todo = Mutex::new(Vec::new());
        let out = dispatch_local(
            &root,
            &todo,
            "table",
            &json!({"path": "revenue", "spec": table_spec(), "source": "q.csv"}),
        )
        .unwrap();
        assert!(out.contains("saved as revenue.csv"), "the extension is added: {out}");
        let csv = std::fs::read_to_string(root.join("revenue.csv")).expect("the file is there");
        assert_eq!(csv, "\"Region\",\"Revenue\"\n\"North\",\"1200\"\n\"South\",\"90\"\n");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    pub(super) fn a_value_with_a_comma_or_a_quote_cannot_corrupt_the_csv() {
        // The case that breaks a CSV silently: the file still parses, just into the
        // wrong number of columns, and whoever opens it has no way to tell.
        let root = scratch();
        let todo = Mutex::new(Vec::new());
        let spec = json!({
            "columns": [{"field": "name"}],
            "rows": [{"name": "North, \"upper\""}, {"name": "line\nbreak"}]
        });
        dispatch_local(&root, &todo, "table", &json!({"path": "x.csv", "spec": spec, "source": "s"})).unwrap();
        let csv = std::fs::read_to_string(root.join("x.csv")).unwrap();
        assert!(csv.contains("\"North, \"\"upper\"\"\""), "quotes are doubled and the field is wrapped: {csv}");
        assert!(csv.contains("\"line\nbreak\""), "a newline stays inside its quoted field: {csv}");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    pub(super) fn a_table_with_nothing_in_it_is_refused() {
        let root = scratch();
        let todo = Mutex::new(Vec::new());
        for bad in [
            json!({"spec": {"columns": [], "rows": [{"a": 1}]}, "source": "s"}),
            json!({"spec": {"columns": [{"field": "a"}], "rows": []}, "source": "s"}),
            json!({"spec": {"columns": [{"field": ""}], "rows": [{"a": 1}]}, "source": "s"}),
        ] {
            let out = dispatch_local(&root, &todo, "table", &bad).unwrap();
            assert!(out.starts_with("error:"), "got: {out}");
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    pub(super) fn a_chart_is_drawn_without_leaving_a_file_behind() {
        // Showing and keeping are different asks. `path` was required, so every chart
        // written into a conversation also became a file nobody had requested — the
        // transcript draws from the tool call itself and never needed one.
        let root = scratch();
        let todo = Mutex::new(Vec::new());
        let out = dispatch_local(
            &root,
            &todo,
            "chart",
            &json!({"spec": chart_spec(), "source": "quarterly.csv"}),
        )
        .unwrap();

        assert!(out.contains("drawn in the conversation"), "got: {out}");
        assert!(out.contains("not saved"), "the model has to know it was not kept: {out}");
        let files: Vec<_> = std::fs::read_dir(&root)
            .map(|rd| rd.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect())
            .unwrap_or_default();
        assert!(files.is_empty(), "nothing should have been written, found: {files:?}");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    pub(super) fn a_chart_is_saved_when_a_path_is_given() {
        let root = scratch();
        let todo = Mutex::new(Vec::new());
        let out = dispatch_local(
            &root,
            &todo,
            "chart",
            &json!({"path": "revenue.chart.json", "spec": chart_spec(), "source": "quarterly.csv"}),
        )
        .unwrap();

        assert!(out.contains("saved as revenue.chart.json"), "got: {out}");
        // Still drawn, not "written instead of shown" — the two are not alternatives.
        assert!(out.contains("drawn in the conversation"), "got: {out}");
        let written = std::fs::read_to_string(root.join("revenue.chart.json")).expect("the file is there");
        // The source is stamped in either way, so a kept chart can still answer where
        // its numbers came from.
        assert!(written.contains("quarterly.csv"), "the provenance has to survive into the file");
        let _ = std::fs::remove_dir_all(&root);
    }
