use super::*;
    use crate::core::agent::RiskClass;

    fn skill(name: &str, tools: &[&str]) -> Skill {
        Skill {
            name: name.into(),
            description: format!("what {name} is for"),
            instructions: "do the thing".into(),
            path: None,
            allowed_tools: tools.iter().map(|t| t.to_string()).collect(),
        }
    }

    fn spec(name: &str) -> ToolSpec {
        ToolSpec {
            name: name.into(),
            description: String::new(),
            schema: serde_json::json!({}),
            risk: RiskClass::Read,
            capability: "test".into(),
        }
    }

    fn names(specs: &[ToolSpec]) -> Vec<&str> {
        specs.iter().map(|s| s.name.as_str()).collect()
    }

    #[test]
    fn a_tool_no_skill_claims_is_always_offered() {
        let sheets = skill("sheet-report", &["sheet_read", "sheet_write"]);
        let review = skill("code-review", &["grep", "read_file"]);
        let skills = [&sheets, &review];
        let specs = [spec("sheet_read"), spec("sheet_write"), spec("grep"), spec("read_file"), spec("web_search")];
        let out = offered(&specs, &skills, &["code-review".into()]);
        // The chosen skill's tools, and `web_search`, which nothing declared.
        assert_eq!(names(&out), vec!["grep", "read_file", "web_search"]);
        // Choosing none still leaves the unclaimed ones.
        assert_eq!(names(&offered(&specs, &skills, &[])), vec!["web_search"]);
    }

    #[test]
    fn a_reply_is_read_through_whatever_the_model_wrapped_it_in() {
        let a = skill("sheet-report", &["sheet_read"]);
        let b = skill("code-review", &["grep"]);
        let skills = [&a, &b];
        assert_eq!(parse(r#"{"skills":["code-review"]}"#, &skills), Some(vec!["code-review".to_string()]));
        assert_eq!(
            parse("Sure!\n```json\n{\"skills\": [\"Code-Review\", \"sheet-report\"]}\n```", &skills),
            Some(vec!["code-review".to_string(), "sheet-report".to_string()]),
            "case is not the test, and fences are how models answer"
        );
        // An invented name is dropped; the rest of the pick stands.
        assert_eq!(parse(r#"{"skills":["code-review","no-such-skill"]}"#, &skills), Some(vec!["code-review".to_string()]));
        // No JSON at all is not a pick — and `None` is what makes the run fail open.
        assert_eq!(parse("I think code-review", &skills), None);
        assert_eq!(parse(r#"{"picked":["code-review"]}"#, &skills), None);
    }

    #[test]
    fn it_fails_open_in_every_way_it_can() {
        let declared = skill("sheet-report", &["sheet_read"]);
        let bare = skill("notes", &[]);
        let few: Vec<ToolSpec> = (0..FLOOR).map(|i| spec(&format!("t{i}"))).collect();
        let mut many: Vec<ToolSpec> = (0..FLOOR + 4).map(|i| spec(&format!("t{i}"))).collect();
        many.push(spec("sheet_read"));

        // Too few tools to be worth narrowing.
        assert_eq!(decide(&few, &[&declared], Some(vec![])).1, Why::UnderFloor);
        // Nothing declares anything, so there is nothing to narrow BY.
        assert_eq!(decide(&many, &[&bare], Some(vec![])).1, Why::NothingDeclared);
        // The chooser could not answer: everything, and said so.
        let (out, why) = decide(&many, &[&declared], None);
        assert_eq!((out.len(), why), (many.len(), Why::Unavailable));
        // It answered "none of these": still everything. An empty pick says the skills
        // don't apply, not that a tool is unwanted — `[]` and a failed chooser are
        // treated identically.
        let (out, why) = decide(&many, &[&declared], Some(vec![]));
        assert_eq!((out.len(), why), (many.len(), Why::NothingChosen));
        // It picked: the claimed tool comes back.
        let (out, why) = decide(&many, &[&declared], Some(vec!["sheet-report".into()]));
        assert_eq!((out.len(), why), (many.len(), Why::Selected));
    }

    #[test]
    fn the_switch_is_off_unless_someone_turns_it_on() {
        assert!(!on() || std::env::var("SUPERUSER_SKILL_NARROWING").is_ok());
    }
