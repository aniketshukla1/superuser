use super::*;

    fn dir(tag: &str) -> PathBuf {
        let n = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let p = std::env::temp_dir().join(format!("bba-feedback-{tag}-{n}"));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn down(session: &str, msg: usize, reason: Reason, model: &str, tools: &[&str]) -> Entry {
        let mut e = entry(session, msg, Verdict::Down);
        e.reason = Some(reason);
        e.evidence = Evidence {
            model: model.into(),
            persona: "assistant".into(),
            tools: tools.iter().map(|t| t.to_string()).collect(),
            steps: 3,
            seconds: 12.0,
        };
        e
    }

    #[test]
    fn a_verdict_round_trips_through_the_file() {
        let d = dir("roundtrip");
        record(&d, &down("s1", 2, Reason::MadeItUp, "gemma4:12b", &["read_file"]));
        let all = all(&d);
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].reason, Some(Reason::MadeItUp));
        assert_eq!(all[0].evidence.tools, vec!["read_file".to_string()]);
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn changing_your_mind_counts_once() {
        // The file is append-only, so both records survive on disk; the report is
        // about what the user currently thinks.
        let d = dir("mind");
        record(&d, &down("s1", 2, Reason::WrongShape, "m", &[]));
        record(&d, &entry("s1", 2, Verdict::Up));
        let entries = all(&d);
        assert_eq!(entries.len(), 2, "nothing is overwritten");
        assert_eq!(verdict_for(&entries, "s1", 2), Some(Verdict::Up), "the later verdict stands");
        let r = report(&entries);
        assert_eq!((r.up, r.down), (1, 0), "one message, one vote");
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn the_report_separates_a_model_problem_from_a_harness_problem() {
        // The whole reason `by_model` exists: "it made things up on every model" and
        // "it made things up on the 3B" are different bugs with different owners.
        let entries = vec![
            down("s1", 1, Reason::MadeItUp, "llama3.2", &["read_file"]),
            down("s2", 1, Reason::MadeItUp, "llama3.2", &["read_file"]),
            down("s3", 1, Reason::WrongSource, "gemma4:12b", &["recall_memory"]),
        ];
        let r = report(&entries);
        assert_eq!(r.down, 3);
        assert_eq!(r.by_reason[0], (Reason::MadeItUp, 2));
        assert_eq!(r.by_model[0], ("llama3.2".to_string(), 2));
        let out = r.render();
        assert!(out.contains("gates.rs"), "a reason has to point somewhere: {out}");
        assert!(out.contains("Presence, not blame"), "the tool column must caveat itself: {out}");
    }

    #[test]
    fn an_upvote_needs_no_taxonomy() {
        let r = report(&[entry("s1", 1, Verdict::Up)]);
        assert_eq!((r.up, r.down), (1, 0));
        assert!(r.by_reason.is_empty(), "nothing to explain about a turn that worked");
    }

    #[test]
    fn every_reason_has_a_label_and_a_place_to_look() {
        // A reason nobody can read is not an option, and one that points nowhere is
        // a mood with a name.
        for r in Reason::ALL {
            assert!(!r.label().is_empty());
            assert!(!r.points_at().is_empty());
            assert_eq!(Reason::parse(r.as_str()), Some(*r), "{} must survive the wire", r.as_str());
        }
        assert_eq!(Reason::parse("nonsense"), None);
    }

    #[test]
    fn a_corrupt_line_does_not_lose_the_rest_of_the_file() {
        let d = dir("corrupt");
        record(&d, &entry("s1", 1, Verdict::Up));
        {
            let mut f = OpenOptions::new().append(true).open(path(&d)).unwrap();
            writeln!(f, "{{not json").unwrap();
        }
        record(&d, &entry("s2", 1, Verdict::Up));
        assert_eq!(all(&d).len(), 2, "one bad line must not cost the file");
        std::fs::remove_dir_all(&d).ok();
    }
