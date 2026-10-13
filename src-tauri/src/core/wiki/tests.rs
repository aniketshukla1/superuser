use super::*;

    fn scratch() -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(0);
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let p = std::env::temp_dir().join(format!("bba-wiki-{}-{}", nanos, N.fetch_add(1, Ordering::SeqCst)));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn im(target: &str, snapshot: &str, note: &str) -> Impact {
        Impact {
            at: stamp(),
            target: target.into(),
            patterns: vec!["changelog-step".into()],
            verdict: Verdict::Accepted,
            snapshot: snapshot.into(),
            note: note.into(),
        }
    }

    #[test]
    fn the_same_observation_twice_reinforces_one_page_instead_of_making_two() {
        // The whole reason a pattern has a stable id: three conversations hitting the
        // same wall must leave one page seen 3×, not three pages seen once. Recurrence
        // is the only signal that says which wall is worth a skill.
        let w = Wiki::new(&scratch());
        assert_eq!(w.upsert_pattern("changelog-step", "Release skips the changelog", Kind::Failure, "first").unwrap(), Wrote::Created);
        assert_eq!(w.upsert_pattern("changelog-step", "Release skips the changelog", Kind::Failure, "second").unwrap(), Wrote::Reinforced);
        assert_eq!(w.upsert_pattern("changelog-step", "", Kind::Failure, "third").unwrap(), Wrote::Reinforced);

        let ps = w.patterns();
        assert_eq!(ps.len(), 1, "one page, not three");
        assert_eq!(ps[0].seen, 3);
        assert_eq!(ps[0].body, "third", "the newest wording wins");
        assert_eq!(ps[0].title, "Release skips the changelog", "an empty title keeps the one already there");
    }

    #[test]
    fn patterns_are_read_back_most_reinforced_first() {
        let w = Wiki::new(&scratch());
        w.upsert_pattern("rare", "Seen once", Kind::Strategy, "x").unwrap();
        for _ in 0..3 {
            w.upsert_pattern("common", "Seen often", Kind::Failure, "y").unwrap();
        }
        let ps = w.patterns();
        assert_eq!(ps.iter().map(|p| p.id.as_str()).collect::<Vec<_>>(), ["common", "rare"]);
        assert_eq!(ps[0].kind, Kind::Failure);
        assert_eq!(ps[1].kind, Kind::Strategy, "a strategy is not silently read back as a failure");
    }

    #[test]
    fn an_id_from_the_model_cannot_escape_the_patterns_directory() {
        // Ids become filenames and the reviewer picks them, so they are untrusted.
        let dir = scratch();
        let w = Wiki::new(&dir);
        w.upsert_pattern("../../../etc/passwd", "nope", Kind::Failure, "body").unwrap();
        assert!(!dir.join("etc").exists());
        let ps = w.patterns();
        assert_eq!(ps.len(), 1);
        assert!(!ps[0].id.contains(".."), "got {:?}", ps[0].id);
        assert!(w.pattern_path(&ps[0].id).starts_with(w.patterns_dir()));
    }

    #[test]
    fn a_title_with_a_pipe_or_a_newline_does_not_corrupt_the_index() {
        // index.md is a markdown table, and the table is the storage.
        let w = Wiki::new(&scratch());
        w.upsert_pattern("odd", "a | b\nc: d", Kind::Failure, "body").unwrap();
        w.rewrite_index().unwrap();
        let idx = std::fs::read_to_string(w.index_path()).unwrap();
        let rows = idx.lines().filter(|l| l.starts_with('|')).count();
        assert_eq!(rows, 3, "header, separator, one pattern — not a row split in two:\n{idx}");
        assert_eq!(w.patterns()[0].title, "a | b c: d", "the colon survived the frontmatter round trip");
    }

    #[test]
    fn the_index_is_regenerated_not_appended() {
        let w = Wiki::new(&scratch());
        w.upsert_pattern("one", "One", Kind::Failure, "b").unwrap();
        w.rewrite_index().unwrap();
        w.upsert_pattern("one", "One", Kind::Failure, "b").unwrap();
        w.rewrite_index().unwrap();
        let idx = std::fs::read_to_string(w.index_path()).unwrap();
        assert_eq!(idx.matches("patterns/one.md").count(), 1, "one row, not a second appended:\n{idx}");
        assert!(idx.contains("2×"), "and it carries the updated count:\n{idx}");
    }

    #[test]
    fn an_undo_flips_the_verdict_and_keeps_the_row() {
        // The point of the layer. The skill goes back; the knowledge that this edit
        // was refused does not.
        let w = Wiki::new(&scratch());
        w.record_impact(&im("release", "evo-01", "added a changelog step")).unwrap();
        w.record_impact(&im("deploy", "evo-02", "unrelated change")).unwrap();

        assert_eq!(w.mark_undone("evo-01"), 1);
        let after = w.impacts();
        assert_eq!(after.len(), 2, "the rejected row survives");
        assert_eq!(after[0].verdict, Verdict::Undone);
        assert_eq!(after[0].note, "added a changelog step", "and still says what it was");
        assert_eq!(after[1].verdict, Verdict::Accepted, "an unrelated snapshot is untouched");
        assert_eq!(w.mark_undone("evo-01"), 0, "undoing twice changes nothing");
    }

    #[test]
    fn a_rejected_change_is_named_in_the_brief_so_it_is_not_proposed_again() {
        let w = Wiki::new(&scratch());
        w.upsert_pattern("changelog-step", "Release skips the changelog", Kind::Failure, "body").unwrap();
        w.record_impact(&im("release", "evo-01", "added a changelog step")).unwrap();
        assert!(!w.brief(20).contains("rejected"), "nothing rejected yet");

        w.mark_undone("evo-01");
        let brief = w.brief(20);
        assert!(brief.contains("changelog-step"), "patterns are shown:\n{brief}");
        assert!(brief.contains("seen 1×"));
        assert!(brief.contains("already rejected"), "the rejection is shown:\n{brief}");
        assert!(brief.contains("added a changelog step"));
    }

    #[test]
    fn the_brief_keeps_the_repeated_patterns_when_it_has_to_drop_some() {
        // It rides in one prompt beside a transcript, so it is bounded — and what
        // survives truncation has to be the recurring evidence, not the one-offs.
        let w = Wiki::new(&scratch());
        for i in 0..10 {
            w.upsert_pattern(&format!("one-off-{i}"), &format!("Seen once {i}"), Kind::Failure, "b").unwrap();
        }
        for _ in 0..5 {
            w.upsert_pattern("recurring", "Seen a lot", Kind::Failure, "b").unwrap();
        }
        let brief = w.brief(3);
        assert!(brief.contains("recurring"), "the repeated one survived:\n{brief}");
        assert!(brief.contains("and 8 less-repeated patterns"));
    }

    #[test]
    fn an_empty_wiki_contributes_nothing_to_the_prompt() {
        // A first-run reviewer should not be handed a heading over an empty list and
        // start inventing patterns to fill it.
        assert_eq!(Wiki::new(&scratch()).brief(20), "");
    }

    #[test]
    fn the_log_records_passes_that_changed_nothing_else() {
        let w = Wiki::new(&scratch());
        w.log("noted 1 pattern, no skill changed").unwrap();
        w.log("nothing to note").unwrap();
        let text = std::fs::read_to_string(w.logs_path()).unwrap();
        assert_eq!(text.lines().filter(|l| l.starts_with("- `")).count(), 2);
        assert!(text.contains("noted 1 pattern"));
    }

    #[test]
    fn commit_writes_the_pages_and_the_index_together() {
        let vault = scratch();
        let written = commit(
            &vault,
            &[
                ("changelog-step".into(), "Release skips the changelog".into(), Kind::Failure, "body".into()),
                ("blank".into(), "Dropped".into(), Kind::Failure, "   ".into()),
            ],
            &[im("release", "evo-01", "note")],
            "one pattern noted",
        );
        assert_eq!(written, ["changelog-step"], "an empty body is not a pattern");
        let w = Wiki::new(&vault);
        assert!(std::fs::read_to_string(w.index_path()).unwrap().contains("changelog-step"));
        assert_eq!(w.impacts().len(), 1);
        assert!(std::fs::read_to_string(w.logs_path()).unwrap().contains("one pattern noted"));
    }

    #[test]
    fn purpose_explains_a_skill_only_when_something_motivated_it() {
        let dir = scratch().join("release");
        write_purpose(&dir, "release", &[], "note").unwrap();
        assert!(!dir.join("PURPOSE.md").exists(), "no patterns, no claim about why");

        write_purpose(&dir, "release", &["changelog-step".into()], "Added the changelog step.").unwrap();
        let text = std::fs::read_to_string(dir.join("PURPOSE.md")).unwrap();
        assert!(text.contains("changelog-step"));
        assert!(text.contains("Added the changelog step."));
    }

    #[test]
    fn a_pattern_file_edited_by_hand_still_reads() {
        // These are plain markdown in the user's vault; someone will edit one.
        let w = Wiki::new(&scratch());
        std::fs::create_dir_all(w.patterns_dir()).unwrap();
        std::fs::write(w.patterns_dir().join("handwritten.md"), "just some prose, no frontmatter\n").unwrap();
        let ps = w.patterns();
        assert_eq!(ps.len(), 1);
        assert_eq!(ps[0].id, "handwritten");
        assert_eq!(ps[0].title, "handwritten", "a title is derived rather than left blank");
        assert_eq!(ps[0].seen, 1);
        assert_eq!(ps[0].kind, Kind::Failure);
    }
