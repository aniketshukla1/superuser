use super::*;

    fn have() -> Vec<String> {
        vec!["read_file".into(), "grep".into()]
    }

    #[test]
    fn a_narrated_read_is_recovered() {
        // The shape llama3.2 actually produces, and the reason the strict version
        // never fired on the sweep it was written for.
        let text = "Running `list_dir` to find the actual file path, I get: {\"name\": \"list_dir\", \"parameters\": {\"path\": \".\"}}";
        let reads = vec!["list_dir".to_string(), "read_file".to_string()];
        let out = salvage_printed_call_with(text, &reads, &reads).expect("a narrated read should run");
        assert_eq!(out[0]["function"]["name"], "list_dir");
    }

    #[test]
    fn a_narrated_write_is_never_recovered() {
        // The line that keeps this safe. Describing a send is not sending, and running
        // one the user never saw proposed would invent an action on their behalf.
        let text = "I will now call {\"name\": \"send_message\", \"parameters\": {\"text\": \"hi\"}} to tell them.";
        let available = vec!["send_message".to_string(), "list_dir".to_string()];
        let reads = vec!["list_dir".to_string()];
        assert!(salvage_printed_call_with(text, &available, &reads).is_none());
    }

    #[test]
    fn prose_with_no_read_only_list_stays_strict() {
        let text = "Running {\"name\": \"list_dir\", \"parameters\": {}} now";
        assert!(salvage_printed_call_with(text, &["list_dir".to_string()], &[]).is_none());
    }

    #[test]
    fn braces_inside_strings_do_not_confuse_the_scan() {
        let text = "Result: {\"name\": \"read_file\", \"parameters\": {\"path\": \"a{b}.md\"}} done";
        let reads = vec!["read_file".to_string()];
        let out = salvage_printed_call_with(text, &reads, &reads).unwrap();
        assert_eq!(out[0]["function"]["arguments"]["path"], "a{b}.md");
    }

    #[test]
    fn a_printed_call_becomes_a_real_one() {
        let out = salvage_printed_call(r#"{"name":"read_file","parameters":{"path":"a.md"}}"#, &have()).unwrap();
        assert_eq!(out[0]["function"]["name"], "read_file");
        assert_eq!(out[0]["function"]["arguments"]["path"], "a.md");
    }

    #[test]
    fn a_fenced_one_is_recovered_too() {
        let text = "```json\n{\"name\": \"grep\", \"arguments\": {\"pattern\": \"todo\"}}\n```";
        let out = salvage_printed_call(text, &have()).unwrap();
        assert_eq!(out[0]["function"]["name"], "grep");
    }

    #[test]
    fn arguments_as_a_json_string_are_parsed() {
        let out = salvage_printed_call(r#"{"name":"read_file","arguments":"{\"path\":\"b.md\"}"}"#, &have()).unwrap();
        assert_eq!(out[0]["function"]["arguments"]["path"], "b.md");
    }

    #[test]
    fn prose_that_merely_mentions_a_call_is_left_alone() {
        // The line that keeps this honest. A model explaining a call it is about to
        // make has not made it, and running it would invent an action the user never
        // asked for and never saw proposed.
        assert!(salvage_printed_call(
            r#"I will call {"name":"read_file","parameters":{"path":"a.md"}} next."#,
            &have()
        )
        .is_none());
    }

    #[test]
    fn a_tool_this_turn_does_not_have_is_not_salvaged() {
        assert!(salvage_printed_call(r#"{"name":"send_message","parameters":{}}"#, &have()).is_none());
    }

    #[test]
    fn an_ordinary_answer_is_not_a_call() {
        assert!(salvage_printed_call("The file says the timeout is 30 seconds.", &have()).is_none());
        assert!(salvage_printed_call(r#"{"answer": 42}"#, &have()).is_none());
    }
