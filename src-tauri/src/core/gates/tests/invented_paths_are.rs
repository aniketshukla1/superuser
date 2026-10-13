use super::*;

    #[test]
    pub(super) fn reserved_hosts_are_matched_as_hosts_not_substrings() {
        assert!(looks_invented("https://example.com/data", ""));
        assert!(looks_invented("http://www.example.org", ""));
        // Somebody's real host that merely contains the string.
        assert!(!looks_invented("https://example.com.acme.io/q3", ""));
        assert!(!looks_invented("https://notexample.com", ""));
    }

    #[test]
    pub(super) fn invented_paths_are_refused_and_real_ones_are_not() {
        assert!(placeholder("read_file", &json!({"path": "path/to/sales.xlsx"}), "").is_some());
        assert!(placeholder("read_file", &json!({"path": "<filename>.csv"}), "").is_some());
        assert!(placeholder("read_file", &json!({"path": "/Users/username/data.csv"}), "").is_some());
        // Files with ordinary names must sail through — a gate that fires on these
        // teaches the model to avoid the tool layer.
        assert!(placeholder("read_file", &json!({"path": "data.csv"}), "").is_none());
        assert!(placeholder("read_file", &json!({"path": "reports/Q3 sales.xlsx"}), "").is_none());
        assert!(placeholder("read_file", &json!({"path": "src/main.rs"}), "").is_none());
    }

    #[test]
    pub(super) fn the_refusal_names_the_tool_to_use_instead() {
        let msg = placeholder("read_file", &json!({"path": "path/to/x.csv"}), "").unwrap();
        assert!(msg.contains("ask_user"), "a refusal that doesn't name the way forward invites a second guess");
        assert!(msg.contains("list_dir"));
    }

    #[test]
    pub(super) fn only_locatable_citations_are_checked() {
        assert_eq!(cited_locators("the figures you gave me"), Vec::<String>::new());
        assert_eq!(cited_locators("Q3 revenue, from memory"), Vec::<String>::new());
        assert_eq!(cited_locators("read from q3.xlsx"), vec!["q3.xlsx"]);
        assert_eq!(cited_locators("https://acme.io/report"), vec!["https://acme.io/report"]);
        // A decimal is not a file.
        assert_eq!(cited_locators("grew by 3.14 percent"), Vec::<String>::new());
    }

    #[test]
    pub(super) fn a_chart_must_say_where_its_numbers_came_from() {
        assert!(grounding("", &[]).is_some());
        assert!(grounding("   ", &[]).is_some());
    }

    #[test]
    pub(super) fn prose_provenance_is_accepted_because_it_is_the_honest_answer() {
        // The user pasted numbers into the chat. There is no file to cite, and
        // demanding one would force the model to invent the very thing being guarded.
        assert!(grounding("the figures in the user's message", &[]).is_none());
    }

    #[test]
    pub(super) fn citing_a_file_this_run_never_opened_is_refused() {
        let msg = grounding("q3_sales.xlsx", &["notes.md".into()]).unwrap();
        assert!(msg.contains("q3_sales.xlsx"));
        assert!(msg.contains("notes.md"), "tell it what it HAS read, or it guesses again");
    }

    #[test]
    pub(super) fn citing_a_file_this_run_did_open_passes() {
        assert!(grounding("q3.xlsx", &["/work/reports/q3.xlsx".into()]).is_none());
        // …and the other way round, since the model cites what the user calls it
        // while `seen` holds the resolved path.
        assert!(grounding("/work/reports/q3.xlsx", &["q3.xlsx".into()]).is_none());
    }

    #[test]
    pub(super) fn argument_order_cannot_disguise_a_repeat() {
        let a = signature("read_file", &json!({"path": "a", "start_line": 1}));
        let b = signature("read_file", &json!({"start_line": 1, "path": "a"}));
        assert_eq!(a, b);
        assert_ne!(a, signature("read_file", &json!({"path": "b", "start_line": 1})));
    }

    #[test]
    pub(super) fn two_failures_are_a_retry_and_three_are_a_loop() {
        assert!(repetition("grep", 0).is_none());
        assert!(repetition("grep", 1).is_none(), "the second attempt is a legitimate retry");
        assert!(repetition("grep", 2).is_some());
        assert!(repetition("grep", 2).unwrap().contains("ask_user"));
    }
