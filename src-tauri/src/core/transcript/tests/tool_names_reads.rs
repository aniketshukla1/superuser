use super::*;

    #[test]
    pub(super) fn tool_names_reads_both_wire_shapes() {
        // A reader that knew only one shape would report "no tools ran" for half the
        // providers, which is the sort of wrong number that gets believed.
        let openai = TurnWire {
            shape: WireShape::Ollama,
            model: "m".into(),
            messages: vec![serde_json::json!({"tool_calls":[{"function":{"name":"read_file"}}]})],
        };
        assert_eq!(openai.tool_names(), vec!["read_file".to_string()]);

        let claude = TurnWire {
            shape: WireShape::Claude,
            model: "m".into(),
            messages: vec![serde_json::json!({"content":[{"type":"tool_use","name":"grep"},{"type":"text","text":"hi"}]})],
        };
        assert_eq!(claude.tool_names(), vec!["grep".to_string()]);

        let none = TurnWire { shape: WireShape::Ollama, model: "m".into(), messages: vec![serde_json::json!({"content":"just text"})] };
        assert!(none.tool_names().is_empty());
    }

    pub(super) fn profile(kind: ProviderKind) -> ModelProfile {
        let mut p = assumed("test");
        p.kind = kind;
        p
    }

    /// An Ollama turn that read a file and then answered.
    pub(super) fn ollama_turn() -> Vec<Value> {
        vec![
            json!({ "role": "assistant", "content": "", "tool_calls": [
                { "id": "", "function": { "name": "read_file", "arguments": { "path": "alpha.md" } } }
            ]}),
            json!({ "role": "tool", "content": "The launch date is 3 October." }),
            json!({ "role": "assistant", "content": "It says the launch date is 3 October." }),
        ]
    }

    pub(super) fn claude_turn() -> Vec<Value> {
        vec![
            json!({ "role": "assistant", "content": [
                { "type": "text", "text": "Let me look." },
                { "type": "tool_use", "id": "tu_1", "name": "read_file", "input": { "path": "alpha.md" } }
            ]}),
            json!({ "role": "user", "content": [
                { "type": "tool_result", "tool_use_id": "tu_1", "content": "The launch date is 3 October." }
            ]}),
        ]
    }

    #[test]
    pub(super) fn a_turn_with_no_tool_calls_is_not_stored_twice() {
        // The assistant message already holds the answer. A second copy of the same
        // prose in every session file buys nothing and costs every save.
        let plain = vec![json!({ "role": "assistant", "content": "Four." })];
        assert!(TurnWire::capture(WireShape::Ollama, "m", &plain).is_none());
        assert!(TurnWire::capture(WireShape::Ollama, "m", &ollama_turn()).is_some());
    }

    #[test]
    pub(super) fn a_call_that_never_got_its_result_is_not_replayed() {
        // The failure this prevents is displaced in time, which is what makes it
        // nasty: the turn that ends at the step limit looks fine, and the NEXT turn
        // 400s because both APIs require every tool call to be answered.
        let cut_short = vec![
            json!({ "role": "assistant", "tool_calls": [
                { "id": "a", "function": { "name": "read_file", "arguments": {} } }
            ]}),
            json!({ "role": "tool", "content": "ok" }),
            json!({ "role": "assistant", "tool_calls": [
                { "id": "b", "function": { "name": "run_shell", "arguments": {} } }
            ]}),
        ];
        let w = TurnWire::capture(WireShape::Ollama, "m", &cut_short).unwrap();
        assert_eq!(w.messages.len(), 2, "the unanswered call is dropped");
        assert!(!has_calls(w.messages.last().unwrap()));
    }

    #[test]
    pub(super) fn a_transcript_of_only_orphaned_calls_is_dropped_entirely() {
        // Repairing down to nothing but results would leave answers with no questions,
        // which no API accepts either.
        let only_a_call = vec![json!({ "role": "assistant", "tool_calls": [
            { "id": "a", "function": { "name": "read_file", "arguments": {} } }
        ]})];
        let w = TurnWire {
            shape: WireShape::Ollama,
            model: "m".into(),
            messages: only_a_call,
        }
        .repair();
        assert!(w.messages.is_empty());
        assert!(!w.replays_natively_into(&profile(ProviderKind::Ollama)));
    }

    #[test]
    pub(super) fn native_replay_only_into_the_shape_that_produced_it() {
        let w = TurnWire::capture(WireShape::Ollama, "llama3.2", &ollama_turn()).unwrap();
        assert!(w.replays_natively_into(&profile(ProviderKind::Ollama)));
        // OpenAI requires `tool_call_id` on every tool message and Ollama does not
        // send one, so this transcript is not the same dialect however similar it
        // looks. Claude is not even the same schema.
        assert!(!w.replays_natively_into(&profile(ProviderKind::OpenAiCompatible)));
        assert!(!w.replays_natively_into(&profile(ProviderKind::Anthropic)));
    }

    #[test]
    pub(super) fn replaying_natively_carries_the_tool_result_the_model_already_has() {
        // The exit criterion, in one assertion: what turn 1 read is in turn 2's
        // prompt, so there is nothing to re-read.
        let w = TurnWire::capture(WireShape::Ollama, "llama3.2", &ollama_turn()).unwrap();
        let hist = vec![
            ("user".into(), "what is in alpha.md?".into(), None),
            ("assistant".into(), "It says the launch date is 3 October.".into(), Some(w)),
        ];
        let msgs = replay(&hist, &profile(ProviderKind::Ollama));
        let flat = serde_json::to_string(&msgs).unwrap();
        assert!(flat.contains("read_file"), "the call survives");
        assert!(flat.contains("3 October"), "and so does what came back");
        assert_eq!(msgs[0]["role"], "user");
    }

    #[test]
    pub(super) fn a_projection_carries_the_content_and_says_it_is_a_summary() {
        // "read_file → 412 chars" does not stop a model re-reading the file. The first
        // part of the file does. And the label is what keeps a summary of the model's
        // own past actions from reading as a tool result arriving now.
        let w = TurnWire::capture(WireShape::Claude, "claude-opus-5", &claude_turn()).unwrap();
        let text = w.project();
        assert!(text.contains("read_file"));
        assert!(text.contains(r#"path="alpha.md""#));
        assert!(text.contains("3 October"), "the result content, not just its size");
        assert!(text.contains("already taken"), "labelled as a summary");
    }

    #[test]
    pub(super) fn switching_models_mid_session_projects_rather_than_breaking() {
        // The reason the flattening existed at all: a Claude-shaped `content` array
        // cannot be handed to Ollama. Model switching has to keep working, and it has
        // to keep working *with* the earlier turn's knowledge, not without it.
        let w = TurnWire::capture(WireShape::Claude, "claude-opus-5", &claude_turn()).unwrap();
        let hist = vec![
            ("user".into(), "what is in alpha.md?".into(), None),
            ("assistant".into(), "The launch is 3 October.".into(), Some(w)),
        ];
        let msgs = replay(&hist, &profile(ProviderKind::Ollama));
        assert_eq!(msgs.len(), 2);
        // Flat strings on both, which is what Ollama can read.
        assert!(msgs.iter().all(|m| m["content"].is_string()));
        let text = msgs[1]["content"].as_str().unwrap();
        assert!(text.contains("read_file") && text.contains("3 October"));
        assert!(text.ends_with("The launch is 3 October."), "the answer is kept too");
    }

    #[test]
    pub(super) fn an_assistant_turn_that_was_only_tool_calls_no_longer_vanishes() {
        // `hist_json` filtered on non-empty content, so a turn that called tools and
        // said nothing disappeared from history completely — the model's own actions,
        // erased. Now the transcript carries it.
        let silent = vec![
            json!({ "role": "assistant", "content": "", "tool_calls": [
                { "id": "", "function": { "name": "run_shell", "arguments": { "command": "ls" } } }
            ]}),
            json!({ "role": "tool", "content": "alpha.md  beta.md" }),
        ];
        let w = TurnWire::capture(WireShape::Ollama, "m", &silent).unwrap();
        let hist = vec![("assistant".to_string(), String::new(), Some(w))];
        let msgs = replay(&hist, &profile(ProviderKind::Ollama));
        assert!(!msgs.is_empty(), "the turn is still there");
        assert!(serde_json::to_string(&msgs).unwrap().contains("alpha.md"));
    }
