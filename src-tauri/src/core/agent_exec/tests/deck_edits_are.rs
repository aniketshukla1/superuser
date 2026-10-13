use super::*;

    #[test]
    pub(super) fn a_bad_chart_is_refused_before_anything_touches_disk() {
        let root = scratch();
        let todo = Mutex::new(Vec::new());
        for bad in [
            json!({"path": "x.chart.json", "spec": {"type": "sunburst", "data": [{"a": 1}]}, "source": "s"}),
            json!({"path": "x.chart.json", "spec": {"type": "bar", "data": []}, "source": "s"}),
        ] {
            let out = dispatch_local(&root, &todo, "chart", &bad).unwrap();
            assert!(out.starts_with("error:"), "got: {out}");
        }
        assert!(!root.join("x.chart.json").exists(), "a refused chart must not leave a file");
        // And the path is still checked when one IS given.
        let out = dispatch_local(
            &root,
            &todo,
            "chart",
            &json!({"path": "../escape.chart.json", "spec": chart_spec(), "source": "s"}),
        )
        .unwrap();
        assert!(out.starts_with("error:"), "got: {out}");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    pub(super) fn document_edits_are_numbered_from_one_and_leave_the_rest_alone() {
        // The same contract as the deck tools: numbered from 1 because that is how
        // the document is numbered and how the person asking counts, and surgical
        // because `doc_write` would flatten a real Word file's styles and tables.
        let root = scratch();
        let todo = Mutex::new(Vec::new());
        let doc = root.join("d.docx");
        crate::core::docwrite::write_docx(
            &doc,
            Some("Report"),
            &["First para.".into(), "Second para.".into(), "Third para.".into()],
        )
        .unwrap();

        let listed = dispatch_local(&root, &todo, "doc_read", &json!({"path":"d.docx"})).unwrap();
        assert!(listed.starts_with("[1] Report"), "listing should start at 1: {listed}");
        assert!(!listed.contains("[0]"), "no zeroth paragraph: {listed}");

        // "Paragraph 3" must reach the third one — "Second para." — not the fourth.
        let out = dispatch_local(
            &root,
            &todo,
            "doc_edit",
            &json!({"path":"d.docx","edits":[{"paragraph":3,"text":"Second para, revised."}]}),
        )
        .unwrap();
        assert!(out.starts_with("changed"), "got: {out}");

        let after = crate::core::docedit::read_paragraphs(&doc).unwrap();
        assert_eq!(
            after,
            vec!["Report", "First para.", "Second para, revised.", "Third para."],
            "only paragraph 3 should have changed"
        );

        let zero = dispatch_local(
            &root,
            &todo,
            "doc_edit",
            &json!({"path":"d.docx","edits":[{"paragraph":0,"text":"nope"}]}),
        )
        .unwrap();
        assert!(zero.contains("numbered from 1"), "got: {zero}");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    pub(super) fn deck_edits_are_numbered_from_one() {
        // A real edit went to the wrong slide because the tool took 0-based indices
        // while the person asking said "slide 2" meaning the second. The listing and
        // the tool now both count from 1, and a 0 is refused rather than quietly
        // treated as the first slide.
        let root = scratch();
        let todo = Mutex::new(Vec::new());
        let deck = root.join("d.pptx");
        crate::core::docwrite::write_pptx(
            &deck,
            &[
                crate::core::docwrite::Slide { title: "First".into(), bullets: vec![] },
                crate::core::docwrite::Slide { title: "Second".into(), bullets: vec![] },
                crate::core::docwrite::Slide { title: "Third".into(), bullets: vec![] },
            ],
        )
        .unwrap();

        // The listing a model reads must start at 1, or it will echo back 0-based.
        let listed = dispatch_local(&root, &todo, "slides_read", &json!({"path":"d.pptx"})).unwrap();
        assert!(listed.starts_with("Slide 1:"), "listing should start at 1: {listed}");
        assert!(!listed.contains("Slide 0:"), "no zeroth slide: {listed}");

        // "Slide 2" must reach the SECOND slide.
        let shape = listed
            .lines()
            .skip_while(|l| !l.starts_with("Slide 2:"))
            .nth(1)
            .and_then(|l| l.trim().strip_prefix('['))
            .and_then(|l| l.split(']').next())
            .and_then(|n| n.parse::<u64>().ok())
            .expect("slide 2 should list a shape");
        let out = dispatch_local(
            &root,
            &todo,
            "slides_edit",
            &json!({"path":"d.pptx","edits":[{"slide":2,"shape":shape,"text":"Second, edited"}]}),
        )
        .unwrap();
        assert!(out.starts_with("changed"), "got: {out}");

        let after = crate::core::pptedit::read_slides(&deck).unwrap();
        assert!(after[1].iter().any(|t| t == "Second, edited"), "slide 2 should have changed: {after:?}");
        assert!(after[0].iter().any(|t| t == "First"), "slide 1 must be untouched: {after:?}");
        assert!(after[2].iter().any(|t| t == "Third"), "slide 3 must be untouched: {after:?}");

        // A 0 is a caller still thinking in offsets.
        let zero = dispatch_local(
            &root,
            &todo,
            "slides_edit",
            &json!({"path":"d.pptx","edits":[{"slide":0,"shape":1,"text":"nope"}]}),
        )
        .unwrap();
        assert!(zero.contains("numbered from 1"), "got: {zero}");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    pub(super) fn write_file_refuses_binary_document_names() {
        let root = scratch();
        let todo = Mutex::new(Vec::new());

        // The real failure this guards: a 58-byte .xlsx of prose that Excel cannot
        // open, written with `write_file` and reported as a success.
        for (path, expect_tool) in [
            ("monthly_budget.xlsx", "sheet_new"),
            ("report.docx", "doc_write"),
            ("deck.pptx", "slides_write"),
        ] {
            let out = dispatch_local(
                &root,
                &todo,
                "write_file",
                &json!({"path": path, "content": "Sheet created for monthly budget."}),
            )
            .unwrap();
            assert!(out.starts_with("error:"), "{path} should be refused, got: {out}");
            assert!(out.contains(expect_tool), "{path} should name {expect_tool}, got: {out}");
            // The refusal must not leave a half-written file behind for the panel
            // to list and fail to preview.
            assert!(!root.join(path).exists(), "{path} should not have been created");
        }

        // Extension matching is case-insensitive — .XLSX is the same trap.
        let upper = dispatch_local(&root, &todo, "write_file", &json!({"path":"Q3.XLSX","content":"x"})).unwrap();
        assert!(upper.starts_with("error:"), "got: {upper}");

        // Text files that merely mention a document are unaffected.
        for ok_path in ["budget.md", "notes.txt", "sheet.xlsx.md", "data.csv"] {
            let out = dispatch_local(&root, &todo, "write_file", &json!({"path": ok_path, "content":"fine"})).unwrap();
            assert!(out.starts_with("wrote"), "{ok_path} should write, got: {out}");
        }

        let _ = std::fs::remove_dir_all(&root);
    }
