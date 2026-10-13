use super::*;

    fn skipped(reason: &str) -> Status {
        Status::Skipped { reason: reason.into() }
    }

    #[test]
    fn the_catalogue_is_closed_and_looked_up_case_insensitively() {
        assert!(find("gitleaks").is_some());
        assert!(find("GitLeaks").is_some());
        assert!(find(" trivy ").is_some());
        // Not a scanner we know how to talk about — and never interpolated into a shell.
        assert!(find("rm").is_none());
        assert!(find("curl; rm -rf /").is_none());
    }

    #[test]
    fn every_catalogue_entry_says_what_is_lost_without_it() {
        for t in CATALOG {
            assert!(!t.fallback.trim().is_empty(), "{} has no fallback line", t.name);
            assert!(!t.install.trim().is_empty(), "{} has no install command", t.name);
            assert!(!t.check.trim().is_empty(), "{} has no check name", t.name);
        }
    }

    #[test]
    fn probing_a_real_machine_answers_one_way_or_the_other() {
        // Whatever is on this machine, the answer is a decision, never a panic.
        let p = probe(find("gitleaks").unwrap());
        match p {
            Presence::Present { version } => assert!(!version.is_empty()),
            Presence::Missing => {}
        }
    }

    #[test]
    fn amending_a_check_replaces_it_rather_than_doubling_it() {
        let mut l = Vec::new();
        record(&mut l, "Secret sweep", skipped("gitleaks is not installed"));
        assert_eq!(l.len(), 1);
        record(&mut l, "Secret sweep", Status::Degraded { reason: "swept by hand with git log -p".into() });
        assert_eq!(l.len(), 1, "one check, one line");
        assert!(matches!(l[0].status, Status::Degraded { .. }));
    }

    #[test]
    fn a_skipped_check_is_the_loudest_line_in_the_note() {
        let mut l = Vec::new();
        record(&mut l, "Static analysis", Status::Ran { tool: "semgrep".into(), version: "1.2".into() });
        record(&mut l, "Secret sweep", skipped("gitleaks is not installed"));
        let note = render(&l);
        let skipped_at = note.find("Secret sweep").unwrap();
        let ran_at = note.find("Static analysis").unwrap();
        assert!(skipped_at < ran_at, "what did not run has to come first:\n{note}");
        assert!(note.contains("NOT RUN"), "{note}");
        // The sentence that stops a green-looking report being read as a clean one.
        assert!(note.contains("only that they were not looked at"), "{note}");
    }

    #[test]
    fn a_clean_run_says_so_without_the_warning_tail() {
        let mut l = Vec::new();
        record(&mut l, "Static analysis", Status::Ran { tool: "semgrep".into(), version: "1.2".into() });
        let note = render(&l);
        assert!(!note.contains("did not run"), "{note}");
    }

    #[test]
    fn an_empty_ledger_does_not_pretend_to_be_coverage() {
        assert!(render(&[]).contains("no checks were registered"));
    }
