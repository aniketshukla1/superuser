use super::*;

    fn scratch() -> std::path::PathBuf {
        static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let seq = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let d = std::env::temp_dir().join(format!("su-docedit-{}-{}-{}", std::process::id(), t, seq));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn sample(dir: &std::path::Path) -> std::path::PathBuf {
        let p = dir.join("doc.docx");
        crate::core::docwrite::write_docx(
            &p,
            Some("Quarterly Report"),
            &["First paragraph.".into(), "Second paragraph.".into(), "Third paragraph.".into()],
        )
        .unwrap();
        p
    }

    #[test]
    fn paragraphs_are_read_in_document_order() {
        let d = scratch();
        let p = sample(&d);
        assert_eq!(
            read_paragraphs(&p).unwrap(),
            vec!["Quarterly Report", "First paragraph.", "Second paragraph.", "Third paragraph."]
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn editing_one_paragraph_leaves_every_other_one_alone() {
        // The whole reason this module exists instead of regenerating the file:
        // `write_docx` would flatten a real Word document. Everything not named in
        // the edit has to come back exactly as it was.
        let d = scratch();
        let p = sample(&d);
        write_paragraphs(&p, &[(2, "Second paragraph, revised.".into())]).unwrap();
        assert_eq!(
            read_paragraphs(&p).unwrap(),
            vec!["Quarterly Report", "First paragraph.", "Second paragraph, revised.", "Third paragraph."]
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn an_edited_heading_keeps_its_formatting() {
        // The title is written bold at size 36. Editing its text must not turn it
        // into body text — that is the difference between an edit and a retype.
        let d = scratch();
        let p = sample(&d);
        write_paragraphs(&p, &[(0, "Annual Report".into())]).unwrap();
        let xml = part(&p, DOC).unwrap();
        let first = &xml[paragraph_spans(&xml)[0].0..paragraph_spans(&xml)[0].1];
        assert!(first.contains("Annual Report"));
        assert!(first.contains("<w:b />"), "bold should survive the edit: {first}");
        assert!(first.contains("w:val=\"36\""), "size should survive the edit: {first}");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn several_edits_at_once_all_land() {
        let d = scratch();
        let p = sample(&d);
        write_paragraphs(&p, &[(1, "One.".into()), (3, "Three.".into())]).unwrap();
        assert_eq!(
            read_paragraphs(&p).unwrap(),
            vec!["Quarterly Report", "One.", "Second paragraph.", "Three."]
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn text_needing_escaping_survives_the_round_trip() {
        let d = scratch();
        let p = sample(&d);
        let tricky = "5 < 6 & \"quoted\" <w:p> not a tag";
        write_paragraphs(&p, &[(1, tricky.into())]).unwrap();
        assert_eq!(read_paragraphs(&p).unwrap()[1], tricky);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn an_index_past_the_end_is_refused_rather_than_half_applied() {
        // A stale index means the caller's view and the file have drifted; writing
        // the other edits anyway would scatter them into the wrong paragraphs.
        let d = scratch();
        let p = sample(&d);
        let before = read_paragraphs(&p).unwrap();
        let err = write_paragraphs(&p, &[(1, "changed".into()), (99, "nope".into())]).unwrap_err();
        assert!(err.contains("no paragraph 100"), "got: {err}");
        assert_eq!(read_paragraphs(&p).unwrap(), before, "nothing should have been written");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn the_rest_of_the_archive_is_carried_across_untouched() {
        // Styles, numbering, fonts and comments all live in other parts. A save that
        // dropped them would produce a file Word repairs or refuses.
        let d = scratch();
        let p = sample(&d);
        let names = |path: &std::path::Path| -> Vec<String> {
            let f = std::fs::File::open(path).unwrap();
            let mut z = zip::ZipArchive::new(f).unwrap();
            let mut v: Vec<String> = (0..z.len()).map(|i| z.by_index(i).unwrap().name().to_string()).collect();
            v.sort();
            v
        };
        let before = names(&p);
        write_paragraphs(&p, &[(1, "edited".into())]).unwrap();
        assert_eq!(names(&p), before);
        assert!(before.iter().any(|n| n == "word/styles.xml"), "sanity: {before:?}");
        let _ = std::fs::remove_dir_all(&d);
    }
