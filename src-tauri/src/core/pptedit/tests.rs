use super::*;

    fn scratch() -> std::path::PathBuf {
        static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let seq = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let d = std::env::temp_dir().join(format!("su-pptedit-{}-{}-{}", std::process::id(), t, seq));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn deck(dir: &std::path::Path) -> std::path::PathBuf {
        let p = dir.join("deck.pptx");
        crate::core::docwrite::write_pptx(
            &p,
            &[
                crate::core::docwrite::Slide { title: "Budget tracker".into(), bullets: vec![] },
                crate::core::docwrite::Slide {
                    title: "Four clear jobs".into(),
                    bullets: vec!["Essentials".into(), "Goals".into()],
                },
            ],
        )
        .unwrap();
        p
    }

    #[test]
    fn every_shape_on_every_slide_is_readable_in_order() {
        let d = scratch();
        let p = deck(&d);
        let slides = read_slides(&p).unwrap();
        assert_eq!(slides.len(), 2, "two slides: {slides:?}");
        assert!(slides[0].iter().any(|t| t == "Budget tracker"), "{:?}", slides[0]);
        assert!(slides[1].iter().any(|t| t == "Four clear jobs"), "{:?}", slides[1]);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn editing_one_shape_leaves_every_other_one_alone() {
        // The whole reason this exists rather than regenerating the deck: a real
        // presentation's layout, theme and images cannot be reproduced, so anything
        // not named in the edit has to come back exactly as it was.
        let d = scratch();
        let p = deck(&d);
        let before = read_slides(&p).unwrap();
        let target = before[1].iter().position(|t| t == "Four clear jobs").unwrap();

        write_slides(&p, &[(1, target, "Give income four jobs".into())]).unwrap();

        let after = read_slides(&p).unwrap();
        assert_eq!(after[0], before[0], "slide 1 should be untouched");
        assert_eq!(after[1][target], "Give income four jobs");
        for (i, t) in before[1].iter().enumerate() {
            if i != target {
                assert_eq!(&after[1][i], t, "shape {i} on slide 2 should be untouched");
            }
        }
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn the_shapes_geometry_and_fill_survive_an_edit() {
        // A slide is a layout. If an edit moved or unstyled the shape it touched, the
        // deck would come back rearranged — worse than not editing at all.
        let d = scratch();
        let p = deck(&d);
        let xml_before = crate::core::docedit::part(&p, "ppt/slides/slide1.xml").unwrap();
        let offs_before: Vec<&str> = xml_before.match_indices("<a:off ").map(|(i, _)| &xml_before[i..i + 40]).collect();

        let idx = read_slides(&p).unwrap()[0].iter().position(|t| t == "Budget tracker").unwrap();
        write_slides(&p, &[(0, idx, "Annual budget".into())]).unwrap();

        let xml_after = crate::core::docedit::part(&p, "ppt/slides/slide1.xml").unwrap();
        let offs_after: Vec<&str> = xml_after.match_indices("<a:off ").map(|(i, _)| &xml_after[i..i + 40]).collect();
        assert_eq!(offs_before, offs_after, "every shape should keep its position");
        assert!(xml_after.contains("Annual budget"));
        assert!(
            xml_after.contains("FBF9F6"),
            "the background shape's fill should survive: {}",
            &xml_after[..200.min(xml_after.len())]
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_multi_line_edit_becomes_separate_paragraphs() {
        let d = scratch();
        let p = deck(&d);
        let idx = read_slides(&p).unwrap()[1].iter().position(|t| t.contains("Essentials")).unwrap();
        write_slides(&p, &[(1, idx, "One\nTwo\nThree".into())]).unwrap();
        assert_eq!(read_slides(&p).unwrap()[1][idx], "One\nTwo\nThree");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn text_needing_escaping_survives_the_round_trip() {
        let d = scratch();
        let p = deck(&d);
        let tricky = "Q1 & Q2 < Q3 \"really\" <a:t>not a tag</a:t>";
        write_slides(&p, &[(0, 0, tricky.into())]).unwrap();
        assert_eq!(read_slides(&p).unwrap()[0][0], tricky);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn an_index_past_the_end_is_refused_rather_than_half_applied() {
        let d = scratch();
        let p = deck(&d);
        let before = read_slides(&p).unwrap();

        let e = write_slides(&p, &[(0, 0, "changed".into()), (9, 0, "nope".into())]).unwrap_err();
        assert!(e.contains("no slide 10"), "got: {e}");
        assert_eq!(read_slides(&p).unwrap(), before, "nothing should have been written");

        let e2 = write_slides(&p, &[(0, 0, "changed".into()), (0, 99, "nope".into())]).unwrap_err();
        assert!(e2.contains("no shape 100"), "got: {e2}");
        assert_eq!(read_slides(&p).unwrap(), before, "nothing should have been written");
        let _ = std::fs::remove_dir_all(&d);
    }
