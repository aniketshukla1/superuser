use super::*;
    use crate::core::personas::Recommendation;

    fn person(id: &str, name: &str, connectors: &[&str]) -> PersonaListItem {
        PersonaListItem {
            id: id.into(),
            name: name.into(),
            icon: String::new(),
            tagline: format!("what {name} does"),
            description: String::new(),
            intro: String::new(),
            starters: vec![],
            family: "knowledge".into(),
            kind: "assistant".into(),
            workspace: "none".into(),
            needs_workspace: false,
            tools: vec![],
            subagents: vec![],
            skills: vec![],
            messaging: false,
            connectors: !connectors.is_empty(),
            recommends: connectors
                .iter()
                .map(|c| Recommendation { kind: "connector".into(), reference: (*c).into(), reason: String::new(), tier: "core".into() })
                .collect(),
            builtin: true,
            enabled: true,
            surfaced: true,
            default: false,
        }
    }

    #[test]
    fn the_only_desk_that_holds_it_gets_it() {
        let cands = vec![person("finance", "Finance", &["stripe"]), person("assistant", "Assistant", &[])];
        assert_eq!(by_system(&cands, "refund the Stripe payment for order 41"), Some("finance".into()));
        // Not a word: a router that fires on "linearly" is a router that reads badly.
        let linear = vec![person("pm", "PM", &["linear"]), person("assistant", "Assistant", &[])];
        assert_eq!(by_system(&linear, "the cost grows linearly with volume"), None);
        // Two desks hold it: not a decision this rule can make.
        let both = vec![person("pm", "PM", &["linear"]), person("eng", "Eng", &["linear"])];
        assert_eq!(by_system(&both, "close the linear issue"), None);
    }

    #[test]
    fn a_reply_is_read_by_id_or_by_name_and_nothing_else() {
        let cands = vec![person("finance", "Finance", &[]), person("assistant", "Assistant", &[])];
        assert_eq!(parse(r#"{"assistant":"finance","confidence":0.9}"#, &cands), Some(("finance".into(), 0.9)));
        assert_eq!(parse("```json\n{\"assistant\": \"Finance\", \"confidence\": 1}\n```", &cands), Some(("finance".into(), 1.0)));
        // An assistant nobody has is not an answer.
        assert_eq!(parse(r#"{"assistant":"accounting","confidence":0.9}"#, &cands), None);
        assert_eq!(parse("finance, probably", &cands), None);
        // A confidence outside the range is clamped rather than believed.
        assert_eq!(parse(r#"{"assistant":"finance","confidence":7}"#, &cands).unwrap().1, 1.0);
    }

    #[test]
    fn every_reason_says_whether_it_was_a_choice_or_a_fallback() {
        assert!(Why::OnlyOne.chose() && Why::System.chose() && Why::Model.chose());
        assert!(!Why::LowConfidence.chose() && !Why::Unavailable.chose() && !Why::NoAnswer.chose());
        for w in [Why::OnlyOne, Why::System, Why::Model, Why::LowConfidence, Why::Unavailable, Why::NoAnswer] {
            assert!(!w.label().is_empty());
        }
    }
