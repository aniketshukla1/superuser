use super::*;

    #[test]
    pub(super) fn a_turn_with_no_transcript_replays_exactly_as_it_did_before() {
        // Every session written before this existed, and every plain turn. The old
        // behaviour has to be the fallback, including dropping empty content.
        let hist = vec![
            ("user".to_string(), "hello".to_string(), None),
            ("assistant".to_string(), "  ".to_string(), None),
            ("assistant".to_string(), "hi".to_string(), None),
        ];
        let msgs = replay(&hist, &profile(ProviderKind::Ollama));
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[1]["content"], "hi");
    }

    #[test]
    pub(super) fn a_turn_cut_short_after_a_tool_result_still_replays_well_formed() {
        // The step limit and Stop both land here: the last thing stored is a tool
        // result the model never got to answer. Replayed as-is that puts two user
        // messages back to back — which Anthropic rejects outright and which reads to
        // any model as a question nobody answered.
        let cut = vec![
            json!({ "role": "assistant", "content": [
                { "type": "tool_use", "id": "t1", "name": "run_shell", "input": { "command": "ls" } }
            ]}),
            json!({ "role": "user", "content": [
                { "type": "tool_result", "tool_use_id": "t1", "content": "alpha.md" }
            ]}),
        ];
        let w = TurnWire::capture(WireShape::Claude, "claude-opus-5", &cut).unwrap();
        let hist = vec![
            ("user".to_string(), "list the files".to_string(), None),
            ("assistant".to_string(), "Reached the step limit.".to_string(), Some(w.clone())),
        ];
        let msgs = replay(&hist, &profile(ProviderKind::Anthropic));
        assert_eq!(msgs.last().unwrap()["role"], "assistant", "a turn ends on the assistant");
        assert!(serde_json::to_string(&msgs).unwrap().contains("alpha.md"));

        // And with no answer recorded at all there is nothing to close it with, so it
        // is projected — text is well formed whatever it contains.
        let hist = vec![("assistant".to_string(), String::new(), Some(w))];
        let msgs = replay(&hist, &profile(ProviderKind::Anthropic));
        assert!(msgs.iter().all(|m| m["content"].is_string()), "projected, not native");
        assert!(msgs[0]["content"].as_str().unwrap().contains("alpha.md"));
    }

    #[test]
    pub(super) fn a_huge_tool_result_is_capped_before_it_reaches_disk() {
        // Sessions are rewritten whole on every append, so an uncapped transcript
        // makes every later save proportional to every earlier turn's tool output.
        let big = "x".repeat(200_000);
        let turn = vec![
            json!({ "role": "assistant", "tool_calls": [
                { "id": "a", "function": { "name": "read_file", "arguments": {} } }
            ]}),
            json!({ "role": "tool", "content": big }),
            json!({ "role": "assistant", "content": "done" }),
        ];
        let w = TurnWire::capture(WireShape::Ollama, "m", &turn).unwrap();
        assert!(w.byte_len() <= TURN_CAP, "stored size is bounded");
        // …and the call it answers is still there, because dropping the result would
        // orphan it.
        assert!(w.messages.iter().any(has_calls));
        assert!(w.messages.iter().any(is_result));
    }

    #[test]
    pub(super) fn ollama_results_pair_with_their_calls_without_an_id() {
        // Ollama's `{role: "tool"}` carries no `tool_call_id`. Results arrive in the
        // order the calls were emitted, so oldest-unanswered is the pairing — not a
        // guess, and the only one available.
        let two = vec![
            json!({ "role": "assistant", "tool_calls": [
                { "function": { "name": "read_file", "arguments": { "path": "alpha.md" } } },
                { "function": { "name": "read_file", "arguments": { "path": "beta.md" } } }
            ]}),
            json!({ "role": "tool", "content": "ALPHA BODY" }),
            json!({ "role": "tool", "content": "BETA BODY" }),
            json!({ "role": "assistant", "content": "read both" }),
        ];
        let w = TurnWire::capture(WireShape::Ollama, "m", &two).unwrap();
        let text = w.project();
        let alpha = text.find("ALPHA BODY").expect("first result present");
        let beta = text.find("BETA BODY").expect("second result present");
        assert!(alpha < beta, "in the order the calls were made");
        assert!(text.find("alpha.md").unwrap() < alpha, "under its own call");
    }

    #[test]
    pub(super) fn openai_string_arguments_are_read_not_printed_raw() {
        // OpenAI sends `arguments` as a JSON string and Ollama as an object. A
        // projection that printed the string would show the model an escaped blob
        // where its own call should be.
        let turn = vec![
            json!({ "role": "assistant", "tool_calls": [
                { "id": "c1", "function": { "name": "read_file", "arguments": "{\"path\":\"alpha.md\"}" } }
            ]}),
            json!({ "role": "tool", "tool_call_id": "c1", "content": "body" }),
            json!({ "role": "assistant", "content": "done" }),
        ];
        let w = TurnWire::capture(WireShape::OpenAi, "gpt-5", &turn).unwrap();
        let text = w.project();
        assert!(text.contains(r#"read_file(path="alpha.md")"#), "got: {text}");
        assert!(!text.contains("\\\""), "no escaped JSON in the summary");
    }

    #[test]
    pub(super) fn a_wire_transcript_survives_a_round_trip_through_the_session_file() {
        // It is stored as JSON in the session record, so anything it cannot represent
        // is lost between the turn that captured it and the turn that reads it.
        let w = TurnWire::capture(WireShape::Claude, "claude-opus-5", &claude_turn()).unwrap();
        let back: TurnWire = serde_json::from_str(&serde_json::to_string(&w).unwrap()).unwrap();
        assert_eq!(back, w);
    }
