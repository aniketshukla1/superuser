use super::*;

    fn cat() -> Vec<(&'static str, String)> {
        vec![("files", "read_file".into()), ("files", "write_file".into()), ("shell", "run_shell".into())]
    }

    #[test]
    fn counts_calls_failures_and_distinct_contexts() {
        let mut t = Tally::new();
        t.record("read_file", false, "task_a");
        t.record("read_file", false, "task_a");
        t.record("read_file", true, "task_b");
        let r = &t.rows()[0];
        assert_eq!((r.calls, r.failures, r.contexts), (3, 1, 2), "forty calls in one session is not forty sessions");
        assert!((r.failure_rate() - 1.0 / 3.0).abs() < 1e-9);
    }

    #[test]
    fn a_blocking_tool_that_was_never_answered_is_not_an_error() {
        // The bug this pins, found against real session logs: folding "proposed and
        // never finished" into the error count reported `ask_user` at a 70% failure
        // rate. A question the user walked away from is the tool doing its job.
        let mut t = Tally::new();
        t.record("ask_user", false, "s1");
        t.record_unfinished("ask_user", "s2");
        t.record_unfinished("ask_user", "s3");
        let r = &t.rows()[0];
        assert_eq!(r.calls, 3);
        assert_eq!(r.failures, 0, "nothing errored");
        assert_eq!(r.unfinished, 2);
        assert_eq!(r.failure_rate(), 0.0, "the rate is about errors, not about waiting");
    }

    #[test]
    fn a_tool_with_no_calls_has_a_rate_of_zero_not_a_nan() {
        // It is formatted into a table; a NaN there reads as a bug in the harness.
        assert_eq!(Row::default().failure_rate(), 0.0);
    }

    #[test]
    fn never_called_names_the_capability_so_unavailable_is_distinguishable() {
        let mut t = Tally::new();
        t.record("read_file", false, "s1");
        let never = t.never_called(&cat());
        assert_eq!(never.len(), 2);
        assert!(never.iter().any(|(c, n)| *c == "shell" && n == "run_shell"));
    }

    #[test]
    fn a_tool_outside_the_catalogue_is_surfaced_not_dropped() {
        // An MCP server the user leans on is the strongest evidence about what the
        // catalogue is missing, and it would otherwise vanish from the report.
        let mut t = Tally::new();
        t.record("notion__search", false, "s1");
        t.record("read_file", false, "s1");
        let off = t.off_catalogue(&cat());
        assert_eq!(off.len(), 1);
        assert_eq!(off[0].name, "notion__search");
    }

    #[test]
    fn failure_detection_matches_the_ui_rule() {
        assert!(is_failure("error: no such file"));
        assert!(is_failure("Unknown tool"));
        assert!(is_failure("the build failed: exit 1"));
        assert!(!is_failure("Showing https://example.com in the browser panel."));
        // The word appearing late in a long successful result must not count.
        assert!(!is_failure(&format!("{}error", " ".repeat(5) .to_string() + &"ok ".repeat(60))));
    }

    #[test]
    fn the_report_calls_out_a_resident_tool_nobody_used() {
        let mut t = Tally::new();
        t.record("read_file", false, "s1");
        let out = t.render("Test", &cat(), &["read_file", "run_shell"]);
        assert!(out.contains("Resident but never called here: run_shell"), "{out}");
        assert!(out.contains("2 of 3 catalogue tools were never called"), "{out}");
    }
