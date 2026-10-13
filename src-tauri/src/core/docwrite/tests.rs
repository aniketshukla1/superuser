use super::*;
    use std::path::PathBuf;

    fn scratch() -> PathBuf {
        static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let seq = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let d = std::env::temp_dir().join(format!("su-docw-{}-{}-{}", std::process::id(), t, seq));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn an_ampersand_does_not_make_the_file_unopenable() {
        // Unescaped, this does not corrupt the words — it corrupts the document.
        assert_eq!(esc("Tom & Jerry <b> \"q\""), "Tom &amp; Jerry &lt;b&gt; &quot;q&quot;");
    }

    #[test]
    fn control_characters_are_dropped_rather_than_written_as_illegal_xml() {
        let out = esc("a\u{0007}b");
        assert_eq!(out, "ab");
        // ...but real whitespace survives.
        assert_eq!(esc("a\nb\tc"), "a\nb\tc");
    }

    #[test]
    fn a_word_document_comes_back_as_the_words_written_into_it() {
        let d = scratch();
        let p = d.join("brief.docx");
        write_docx(&p, Some("Quarterly brief"), &["First point.".into(), "Second point.".into()]).unwrap();
        let text = crate::core::files::extract(&p.to_string_lossy()).unwrap().text.unwrap_or_default();
        assert!(text.contains("Quarterly brief"), "title missing: {text}");
        assert!(text.contains("First point.") && text.contains("Second point."), "body missing: {text}");
    }

    #[test]
    fn a_deck_comes_back_as_the_slides_written_into_it() {
        let d = scratch();
        let p = d.join("deck.pptx");
        write_pptx(
            &p,
            &[
                Slide { title: "Renewals".into(), bullets: vec!["Notion — 12 USD".into(), "Linear — 8 USD".into()] },
                Slide { title: "Next steps".into(), bullets: vec!["Cancel unused".into()] },
            ],
        )
        .unwrap();
        let text = crate::core::files::extract(&p.to_string_lossy()).unwrap().text.unwrap_or_default();
        for want in ["Renewals", "Notion", "Linear", "Next steps", "Cancel unused"] {
            assert!(text.contains(want), "`{want}` missing from the deck: {text}");
        }
    }

    #[test]
    fn a_deck_declares_every_part_it_contains() {
        // A part present in the zip but missing from [Content_Types].xml is the
        // classic reason PowerPoint refuses a file outright.
        let d = scratch();
        let p = d.join("deck.pptx");
        write_pptx(&p, &[Slide { title: "One".into(), bullets: vec![] }, Slide { title: "Two".into(), bullets: vec![] }]).unwrap();
        let f = std::fs::File::open(&p).unwrap();
        let mut zip = zip::ZipArchive::new(f).unwrap();
        let names: Vec<String> = (0..zip.len()).filter_map(|i| zip.by_index(i).ok().map(|f| f.name().to_string())).collect();
        for required in [
            "[Content_Types].xml",
            "_rels/.rels",
            "ppt/presentation.xml",
            "ppt/_rels/presentation.xml.rels",
            "ppt/slideMasters/slideMaster1.xml",
            "ppt/slideLayouts/slideLayout1.xml",
            "ppt/theme/theme1.xml",
            "ppt/slides/slide1.xml",
            "ppt/slides/_rels/slide1.xml.rels",
            "ppt/slides/slide2.xml",
        ] {
            assert!(names.iter().any(|n| n == required), "missing part: {required}");
        }
        let mut ct = String::new();
        use std::io::Read;
        zip.by_name("[Content_Types].xml").unwrap().read_to_string(&mut ct).unwrap();
        assert!(ct.contains("/ppt/slides/slide1.xml") && ct.contains("/ppt/slides/slide2.xml"), "a slide is undeclared");
    }

    #[test]
    fn an_empty_slide_still_has_a_paragraph() {
        // A text shape with no <a:p> reads as malformed rather than as empty.
        let xml = slide_xml(&Slide { title: "Only a title".into(), bullets: vec![] });
        assert!(xml.contains("<a:p/>"));
    }

    #[test]
    fn a_deck_with_no_slides_is_refused_rather_than_written_empty() {
        let d = scratch();
        assert!(write_pptx(&d.join("empty.pptx"), &[]).unwrap_err().contains("at least one slide"));
    }
