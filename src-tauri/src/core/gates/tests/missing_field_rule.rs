use super::*;

    #[test]
    pub(super) fn the_recall_rules_appear_only_when_there_is_a_memory_tool() {
        let with = prompt_block(false, false, false, false, true);
        assert!(with.contains("trust the one recorded more recently"), "{with}");
        // The scope clause is the fix for `compare_two_files`; it must not be dropped.
        assert!(with.contains("only about which memory to believe"), "{with}");
        assert!(with.contains("recall_memory` again with that name"), "the two-hop nudge: {with}");
        let without = prompt_block(true, true, true, true, false);
        assert!(!without.contains("recorded more recently"), "a run with no memory tool must not be told about memories");
    }

    #[test]
    pub(super) fn the_provenance_rule_rides_with_the_web_tools_and_only_stops_web_searching() {
        let with = prompt_block(false, false, true, false, false);
        assert!(with.contains("say it is unverified"), "{with}");
        assert!(with.contains("on the web"), "scoped to the web, so it can't argue with a second memory hop: {with}");
        let without = prompt_block(true, true, false, true, true);
        assert!(!without.contains("unverified"), "no web tools, no rule about searching the web: {without}");
    }

    #[test]
    pub(super) fn groping_fires_only_after_a_second_miss() {
        assert!(groping("read_file", 0).is_none());
        assert!(groping("read_file", 1).is_none(), "one wrong guess is an honest guess");
        let msg = groping("read_file", 2).expect("two in a row is a habit");
        assert!(msg.contains("list_dir") && msg.contains("grep"), "it has to name the way out: {msg}");
        assert!(msg.contains("read_file"), "and the tool to retry: {msg}");
    }

    #[test]
    pub(super) fn a_look_is_what_resets_the_count() {
        assert!(is_a_look("list_dir"));
        assert!(is_a_look("grep"));
        assert!(!is_a_look("read_file"));
        assert!(!is_a_look("write_file"));
    }

    #[test]
    pub(super) fn only_a_missing_locator_counts_as_a_miss() {
        // Narrow on purpose. Counting every error would turn this into a general
        // failure gate, and a gate that fires on unrelated failures is one people
        // switch off.
        assert!(is_a_miss("error: no such path: reports/q4.csv. `reports` exists and it holds: q3.csv"));
        assert!(is_a_miss("error: couldn't open /tmp/x/sales.xlsx"));
        assert!(!is_a_miss("error: permission denied"));
        assert!(!is_a_miss("error: .docx is a binary document format"));
        assert!(!is_a_miss("ok, read 40 lines"));
    }

    #[test]
    pub(super) fn urls_come_out_of_prose_without_the_punctuation_around_them() {
        assert_eq!(urls_in("see https://acme.io/q3."), vec!["https://acme.io/q3"]);
        assert_eq!(urls_in("[the report](https://acme.io/q3)"), vec!["https://acme.io/q3"]);
        assert_eq!(urls_in("`https://acme.io/q3`"), vec!["https://acme.io/q3"]);
        assert_eq!(urls_in("a, b: https://acme.io/x?y=1#z!"), vec!["https://acme.io/x?y=1#z"]);
        // The same link twice is one citation.
        assert_eq!(urls_in("https://a.io/x and https://a.io/x"), vec!["https://a.io/x"]);
    }

    #[test]
    pub(super) fn the_word_http_in_prose_is_not_a_citation() {
        assert!(urls_in("we speak http and https here").is_empty());
        assert!(urls_in("http://").is_empty(), "too short to be a destination");
        assert!(urls_in("no links at all").is_empty());
    }

    #[test]
    pub(super) fn technical_prose_is_never_mistaken_for_a_citation() {
        // The reason this gate reads URLs and not paths. Every one of these would
        // pass a filename-shaped rule, and every one of them is ordinary writing.
        for line in [
            "Node.js handles it",
            "call numpy.array on the frame",
            "set self.value = 3",
            "upgrade to 2.14.1",
            "in main.rs, roughly",
            "e.g. the first one",
        ] {
            assert!(urls_in(line).is_empty(), "flagged: {line}");
        }
    }

    #[test]
    pub(super) fn a_link_the_run_never_opened_is_caught() {
        let bad = uncited_urls("Full details at https://acme.io/2026-report.", &[], "");
        assert_eq!(bad, vec!["https://acme.io/2026-report"]);
    }

    #[test]
    pub(super) fn a_link_the_run_did_open_passes() {
        let seen = vec!["https://acme.io/2026-report".to_string()];
        assert!(uncited_urls("Details at https://acme.io/2026-report.", &seen, "").is_empty());
        // …and a deeper anchor into the same page is the same page.
        assert!(uncited_urls("see https://acme.io/2026-report#revenue", &seen, "").is_empty());
    }

    #[test]
    pub(super) fn a_link_the_user_typed_is_theirs_to_cite() {
        // Same exemption as the placeholder gate, for the same reason: the model did
        // not invent what it was handed.
        let said = "summarise https://acme.io/2026-report for me";
        assert!(uncited_urls("As https://acme.io/2026-report says…", &[], said).is_empty());
    }

    #[test]
    pub(super) fn a_link_that_came_back_from_a_search_counts_as_seen() {
        // The weaker bar, deliberately: the run never read the page, but a link a real
        // search returned demonstrably exists, and refusing it would be crying wolf.
        let seen = vec!["https://acme.io/press/q3".to_string()];
        assert!(uncited_urls("There's also https://acme.io/press/q3", &seen, "").is_empty());
    }

    #[test]
    pub(super) fn the_rebuke_says_what_to_do_with_the_sentence() {
        // The other refusals name the next tool call. This one arrives after the work
        // is done, so it has to name what to do with the words already written.
        let msg = citation_rebuke(&["https://acme.io/x".to_string()]);
        assert!(msg.contains("https://acme.io/x"));
        assert!(msg.contains("web_fetch"));
        assert!(msg.contains("take it out"), "rewriting has to be an option, or it will fetch junk");
    }

    /// The missing-field rule appears only for a run that can read a source, and
    /// never names anything from the task that exposed the gap.
    #[test]
    pub(super) fn the_missing_field_rule_is_general_and_gated_on_reading() {
        let off = prompt_block(true, true, true, false, false);
        assert!(!off.contains("not in the source"), "must not appear without reading");

        let on = prompt_block(false, false, false, true, false);
        assert!(on.contains("not in the source"));
        assert!(on.contains("does not say"), "must name the way forward");

        // Instance-specific fixes are what the protocol forbids and what a prompt
        // makes easy. The illustrative example must come from a different world than
        // the task that found the failure, or this is teaching to the test.
        for leaked in ["escalation", "roster", "on-call", "on call", "backup", "marcus", "priya"] {
            assert!(
                !on.to_lowercase().contains(leaked),
                "the rule leaks the task it was written for: {leaked}"
            );
        }
    }

    #[test]
    pub(super) fn the_prompt_block_is_omitted_when_neither_tool_is_granted() {
        // An assistant told about a tool it does not have is an assistant that will try
        // to call it and lose a step to "unknown tool".
        assert_eq!(prompt_block(false, false, false, false, false), "");
        assert!(!prompt_block(false, true, false, false, false).contains("ask_user"));
        assert!(!prompt_block(true, true, false, false, false).contains("fabricated source"));
        // The chart RULE, not the word: "a chart, a report, a new file" is a fine
        // example of a deliverable to check the shape of, whether or not this
        // assistant can draw one. What must not appear is the `source` requirement
        // of a tool it hasn't got.
        assert!(!prompt_block(true, false, false, false, false).contains("`source`"));
    }

    #[test]
    pub(super) fn the_prompt_and_the_refusals_name_the_same_rule() {
        // The two halves have to agree: it is not a gate if the model is told one
        // thing up front and refused for another.
        let p = prompt_block(true, true, true, true, false);
        assert!(p.contains("source"), "the chart rule the gate enforces is not stated");
        assert!(p.contains("ask_user"));
        assert!(grounding("", &[]).unwrap().contains("source"));
        assert!(p.contains("Only link to pages you actually opened"));
        assert!(citation_rebuke(&["https://a.io/x".into()]).contains("never opened"));
    }

    #[test]
    pub(super) fn a_path_the_user_typed_is_never_a_placeholder() {
        // The whole reason the user's words are passed in: "example.com" is a
        // placeholder when the model reached for it, and a destination when the
        // person asked for it by name.
        assert!(looks_invented("https://example.com", "chart my sales"));
        assert!(!looks_invented("https://example.com", "please fetch https://example.com"));
    }
