use super::*;

    fn schema() -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {"type": "string"},
                "occurrence": {"type": "integer"},
            },
            "required": ["path", "occurrence"],
        })
    }

    #[test]
    fn the_body_never_carries_tools_because_tools_suppress_the_grammar() {
        let b = body("llama3.2:latest", &schema(), "edit_file", "Edit a file.", "fix it", &json!({}), "why");
        assert!(b.get("tools").is_none(), "tools and format do not compose: {b}");
        assert_eq!(b["format"], schema());
    }

    #[test]
    fn thinking_is_pinned_off() {
        // 322s against 8.6s on gemma4:12b. This assertion is the whole finding.
        let b = body("gemma4:12b", &schema(), "edit_file", "Edit a file.", "fix it", &json!({}), "why");
        assert_eq!(b["think"], json!(false));
    }

    #[test]
    fn the_instruction_carries_what_was_wrong_and_what_was_asked() {
        let b = body(
            "llama3.2:latest",
            &schema(),
            "edit_file",
            "Edit a file.",
            "replace the second draft",
            &json!({"path": "notes.txt", "occurrence": "2"}),
            "\"occurrence\" must be integer, got string",
        );
        let msg = b["messages"][0]["content"].as_str().unwrap();
        assert!(msg.contains("edit_file"));
        assert!(msg.contains("replace the second draft"));
        assert!(msg.contains("\"occurrence\":\"2\""), "the bad arguments are quoted back: {msg}");
        assert!(msg.contains("must be integer"));
    }

    #[test]
    fn arguments_that_never_parsed_are_not_quoted_back_as_an_empty_object() {
        // `read_arguments` substitutes `{}` for a call whose text never parsed. Telling
        // the model it sent `{}` would be describing a call it did not make.
        let b = body(
            "llama3.2:latest",
            &schema(),
            "edit_file",
            "Edit a file.",
            "replace the second draft",
            &json!({}),
            "the arguments for this call were not valid JSON (EOF while parsing)",
        );
        let msg = b["messages"][0]["content"].as_str().unwrap();
        assert!(!msg.contains("arguments that were sent"), "{msg}");
        assert!(msg.contains("not valid JSON"));
        assert!(msg.contains("replace the second draft"));
    }

    #[test]
    fn a_local_model_is_kept_warm_for_the_step_that_follows() {
        let b = body("llama3.2:latest", &schema(), "t", "d", "a", &json!({}), "w");
        assert!(b.get("keep_alive").is_some());
    }

    #[test]
    fn a_good_object_is_accepted() {
        let out = accept(&schema(), r#"{"path": "notes.txt", "occurrence": 2}"#);
        assert_eq!(out, Some(json!({"path": "notes.txt", "occurrence": 2})));
    }

    #[test]
    fn prose_from_a_server_that_ignored_the_schema_is_not_arguments() {
        assert!(accept(&schema(), "Sure! You'll want to edit notes.txt.").is_none());
    }

    #[test]
    fn a_repair_that_still_violates_the_schema_is_refused() {
        // The grammar's guarantee is about the schema we sent. If the two ever drift
        // apart, this is where it surfaces, rather than at dispatch.
        assert!(accept(&schema(), r#"{"path": "notes.txt", "occurrence": "2"}"#).is_none());
        assert!(accept(&schema(), r#"{"path": "notes.txt"}"#).is_none());
    }

    #[test]
    fn whitespace_around_the_object_does_not_lose_a_good_repair() {
        assert!(accept(&schema(), "\n  {\"path\": \"a\", \"occurrence\": 1}\n ").is_some());
    }

    // ---- Against a real server ------------------------------------------------
    //
    // Everything above asserts the shape of a request; none of it can say whether a
    // model actually answers one. That needs Ollama running, so it lives behind
    // `#[ignore]` alongside the `harness_eval` driver and is run deliberately:
    //
    //     cargo test --lib argrepair -- --ignored --nocapture
    //
    // The task is the one from the measurement: a schema whose `occurrence` is an
    // integer, which `llama3.2:latest` was observed to send as a string 3 times out of
    // 3. If this passes, the loop's repair path works end to end on the model that
    // needed it.
    #[tokio::test]
    #[ignore]
    async fn a_live_model_repairs_an_argument_of_the_wrong_type() {
        let spec = crate::core::agent::ToolSpec {
            name: "edit_file".into(),
            description: "Replace one occurrence of a string in a file.".into(),
            schema: json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string"},
                    "old": {"type": "string"},
                    "new": {"type": "string"},
                    "occurrence": {"type": "integer"},
                },
                "required": ["path", "old", "new", "occurrence"],
            }),
            risk: crate::core::agent::RiskClass::WriteLocal,
            capability: "files".into(),
        };
        let model = std::env::var("ARGREPAIR_MODEL").unwrap_or_else(|_| "llama3.2:latest".into());
        let rep = Repair {
            model,
            url: std::env::var("OLLAMA_URL")
                .unwrap_or_else(|_| "http://localhost:11434".into())
                .trim_end_matches('/')
                .to_string()
                + "/api/chat",
        };
        let http = crate::core::egress::client("model");
        let bad = json!({"path": "notes.txt", "old": "draft", "new": "final", "occurrence": "2"});
        let why = crate::core::loopshape::check_args(&spec.schema, &bad).unwrap_err();
        assert!(why.contains("integer"), "the fixture must actually be rejected: {why}");

        let began = std::time::Instant::now();
        let out = attempt(
            &http,
            &rep,
            &spec,
            "In notes.txt, replace the second occurrence of the word 'draft' with 'final'.",
            &bad,
            &why,
        )
        .await;

        let Some(fixed) = out else {
            panic!("no repair came back — is Ollama running, and is {} pulled?", rep.model);
        };
        println!("repaired in {:.1}s: {fixed}", began.elapsed().as_secs_f64());
        // The point of the whole path: what comes back passes the check that refused
        // the original.
        crate::core::loopshape::check_args(&spec.schema, &fixed).expect("repair must satisfy the schema");
        assert_eq!(fixed["occurrence"], json!(2), "the integer is the thing being fixed");
        // Values the model already had right are not to be re-invented.
        assert_eq!(fixed["old"], json!("draft"));
        assert_eq!(fixed["new"], json!("final"));
    }
