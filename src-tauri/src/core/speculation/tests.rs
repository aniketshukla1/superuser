use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn scratch() -> PathBuf {
        static N: AtomicUsize = AtomicUsize::new(0);
        let d = std::env::temp_dir().join(format!(
            "su-spec-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        let _ = std::fs::create_dir_all(&d);
        d
    }

    #[test]
    fn the_top_hit_is_the_first_path_line_colon_number() {
        let r = "src/a.rs:12:fn needle()\nsrc/b.rs:40:needle again\n";
        assert_eq!(top_hit(r).as_deref(), Some("src/a.rs"));
    }

    #[test]
    fn a_result_with_no_recognisable_hit_speculates_on_nothing() {
        // Not a general parser on purpose: an unrecognised shape means no guess,
        // which is the safe direction.
        assert_eq!(top_hit("no matches found"), None);
        assert_eq!(top_hit("error: bad pattern"), None);
        assert_eq!(top_hit(""), None);
    }

    #[test]
    fn a_path_that_leaves_the_workspace_is_refused_before_anything_opens() {
        let root = Path::new("/tmp/ws");
        assert!(inside(root, "../../etc/passwd").is_none());
        assert!(inside(root, "/etc/passwd").is_none());
        assert!(inside(root, "src/a.rs").is_some());
    }

    #[test]
    fn a_grep_hit_is_prefetched_and_served_to_the_matching_read() {
        let root = scratch();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src/a.rs"), "fn needle() {}").unwrap();
        let mut s = Speculator::new();
        s.consider(&root, "grep", "src/a.rs:1:fn needle()\n", false);
        assert_eq!(s.started(), 1);
        assert_eq!(s.take("read_file", "src/a.rs").as_deref(), Some("fn needle() {}"));
        assert_eq!(s.hits(), 1);
    }

    #[test]
    fn asking_for_a_different_file_gets_a_normal_read_and_costs_only_the_guess() {
        let root = scratch();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src/a.rs"), "a").unwrap();
        let mut s = Speculator::new();
        s.consider(&root, "grep", "src/a.rs:1:x\n", false);
        assert_eq!(s.take("read_file", "src/b.rs"), None);
        assert_eq!(s.misses(), 1);
        assert_eq!(s.hits(), 0);
    }

    #[test]
    fn a_tainted_run_never_has_a_read_made_on_its_behalf() {
        // The thing choosing what to prefetch is downstream of the stranger's text,
        // so the choice is not ours to act on.
        let root = scratch();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src/a.rs"), "a").unwrap();
        let mut s = Speculator::new();
        s.consider(&root, "grep", "src/a.rs:1:x\n", true);
        assert_eq!(s.started(), 0);
        assert_eq!(s.take("read_file", "src/a.rs"), None);
    }

    #[test]
    fn only_grep_shaped_calls_start_a_guess() {
        let root = scratch();
        std::fs::write(root.join("a.rs"), "a").unwrap();
        let mut s = Speculator::new();
        s.consider(&root, "run_shell", "a.rs:1:x\n", false);
        assert_eq!(s.started(), 0, "a shell result is not a mined pattern");
    }

    #[test]
    fn a_write_invalidates_a_guess_rather_than_serving_a_stale_file() {
        let root = scratch();
        std::fs::write(root.join("a.rs"), "before").unwrap();
        let mut s = Speculator::new();
        s.consider(&root, "grep", "a.rs:1:x\n", false);
        s.invalidate();
        assert_eq!(s.take("read_file", "a.rs"), None, "must not serve pre-write contents");
    }

    #[test]
    fn a_file_too_big_to_be_worth_guessing_is_left_alone() {
        let root = scratch();
        let big = "x".repeat((MAX_PREFETCH_BYTES + 1) as usize);
        std::fs::write(root.join("big.log"), &big).unwrap();
        let mut s = Speculator::new();
        s.consider(&root, "grep", "big.log:1:x\n", false);
        assert_eq!(s.started(), 0);
    }

    #[test]
    fn one_guess_at_a_time() {
        let root = scratch();
        std::fs::write(root.join("a.rs"), "a").unwrap();
        std::fs::write(root.join("b.rs"), "b").unwrap();
        let mut s = Speculator::new();
        s.consider(&root, "grep", "a.rs:1:x\n", false);
        s.consider(&root, "grep", "b.rs:1:x\n", false);
        assert_eq!(s.started(), 1, "PREFETCH_LIMIT is one for a reason");
        assert_eq!(s.take("read_file", "a.rs").as_deref(), Some("a"));
    }

    #[test]
    fn a_guess_answers_at_most_one_call() {
        let root = scratch();
        std::fs::write(root.join("a.rs"), "a").unwrap();
        let mut s = Speculator::new();
        s.consider(&root, "grep", "a.rs:1:x\n", false);
        assert!(s.take("read_file", "a.rs").is_some());
        assert!(s.take("read_file", "a.rs").is_none(), "the hold is consumed");
    }
