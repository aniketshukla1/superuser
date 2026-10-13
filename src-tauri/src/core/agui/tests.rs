use super::*;

    fn input() -> Value {
        json!({
            "threadId": "t-1",
            "runId": "r-1",
            "messages": [
                {"role": "system", "content": "you are a bot"},
                {"role": "user", "content": "what is our runway?"},
                {"role": "assistant", "content": "About 14 months."},
                {"role": "user", "content": [{"type": "text", "text": "and after the hire?"}]}
            ]
        })
    }

    #[test]
    fn a_run_is_asked_the_last_thing_a_person_said_and_seeded_with_the_rest() {
        assert_eq!(last_user_message(&input()).as_deref(), Some("and after the hire?"));
        assert_eq!(
            history(&input()),
            vec![("user".to_string(), "what is our runway?".to_string()), ("assistant".to_string(), "About 14 months.".to_string())],
            "the system message is theirs, and the last user message is the ask rather than history"
        );
        // Nothing to answer is not an empty string to answer.
        assert_eq!(last_user_message(&json!({"messages": [{"role": "user", "content": "  "}]})), None);
    }

    #[test]
    fn text_opens_once_and_closes_once() {
        let ids = Ids::from_input(&input());
        let mut open = false;
        let first = events_for(&json!({"type": "text", "text": "About "}), &ids, &mut open);
        assert_eq!(first.len(), 2, "the first delta opens the message");
        assert_eq!(first[0]["type"], "TEXT_MESSAGE_START");
        assert_eq!(first[1]["delta"], "About ");
        let second = events_for(&json!({"type": "text", "text": "9 months."}), &ids, &mut open);
        assert_eq!(second.len(), 1, "and the next one does not open it again");
        assert_eq!(text_end(&ids, open).unwrap()["type"], "TEXT_MESSAGE_END");
        assert!(text_end(&ids, false).is_none(), "nothing to close when nothing opened");
        // An empty delta is not an event.
        assert!(events_for(&json!({"type": "text", "text": ""}), &ids, &mut open).is_empty());
    }

    #[test]
    fn a_step_is_reported_whole_and_never_offered_to_be_run_elsewhere() {
        let ids = Ids::from_input(&input());
        let mut open = true;
        let events = events_for(&json!({"type": "tool", "id": "c1", "name": "read_file", "arguments": {"path": "a.md"}}), &ids, &mut open);
        assert_eq!(events.len(), 3);
        assert_eq!(events[0]["toolCallName"], "read_file");
        assert_eq!(events[0]["parentMessageId"], ids.message);
        assert_eq!(events[1]["delta"], "{\"path\":\"a.md\"}");
        assert_eq!(events[2]["type"], "TOOL_CALL_END");
        // A call with no arguments still says so in a shape a surface can parse.
        let bare = events_for(&json!({"type": "tool", "id": "c2", "name": "list_dir"}), &ids, &mut open);
        assert_eq!(bare[1]["delta"], "{}");
        // Kinds AG-UI has no place for are dropped rather than invented.
        assert!(events_for(&json!({"type": "usage", "text": "1200"}), &ids, &mut open).is_empty());
    }

    #[test]
    fn a_run_that_streamed_nothing_still_sends_its_answer() {
        let ids = Ids::from_input(&input());
        let whole = whole_message(&ids, "About 9 months.");
        assert_eq!(whole.len(), 3);
        assert_eq!(whole[1]["delta"], "About 9 months.");
        assert!(whole_message(&ids, "   ").is_empty());
    }

    #[test]
    fn the_frame_is_the_protocol_and_an_ended_run_always_says_why() {
        let ids = Ids::from_input(&input());
        assert_eq!(run_started(&ids)["threadId"], "t-1");
        assert_eq!(run_finished(&ids)["runId"], "r-1");
        assert_eq!(run_error(&ids, "no model configured")["message"], "no model configured");
        assert_eq!(sse(&json!({"type": "RUN_FINISHED"})), "data: {\"type\":\"RUN_FINISHED\"}\n\n");
        // A caller with no run id still gets one: every event carries it.
        let minted = Ids::from_input(&json!({"threadId": "t"}));
        assert!(minted.run.starts_with("run_") && minted.message.starts_with("msg_run_"));
    }
