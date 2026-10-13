use super::*;
    use serde_json::json;

    #[test]
    fn the_documented_shape_parses_with_per_task_types() {
        let got = parse_subagent_tasks(&json!({"tasks": [
            {"task": "research vendor A", "type": "explore"},
            {"task": "research vendor B", "type": "explore"},
        ]}));
        assert_eq!(got.len(), 2);
        assert_eq!(got[0], ("research vendor A".into(), "explore".into()));
        assert_eq!(got[1].1, "explore");
    }

    #[test]
    fn a_missing_type_defaults_rather_than_failing() {
        let got = parse_subagent_tasks(&json!({"tasks": [{"task": "do the thing"}]}));
        assert_eq!(got, vec![("do the thing".into(), "general-purpose".into())]);
        // An explicitly empty type is the same as omitting it.
        assert_eq!(parse_subagent_tasks(&json!({"tasks": [{"task": "x", "type": ""}]}))[0].1, "general-purpose");
    }

    #[test]
    fn the_shapes_models_actually_emit_are_accepted() {
        // A single task, no array — the most common improvisation.
        assert_eq!(parse_subagent_tasks(&json!({"task": "check the logs"})), vec![("check the logs".into(), "general-purpose".into())]);
        assert_eq!(parse_subagent_tasks(&json!({"task": "check", "type": "explore"}))[0].1, "explore");
        // A bare array of strings.
        let got = parse_subagent_tasks(&json!({"tasks": ["first", "second"]}));
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].0, "first");
    }

    #[test]
    fn blank_and_malformed_calls_yield_nothing_to_run() {
        assert!(parse_subagent_tasks(&json!({})).is_empty());
        assert!(parse_subagent_tasks(&json!({"tasks": []})).is_empty());
        assert!(parse_subagent_tasks(&json!({"task": "   "})).is_empty());
        assert!(parse_subagent_tasks(&json!({"tasks": [{"task": "  "}, {"nope": 1}]})).is_empty());
        // One good entry among junk still runs.
        assert_eq!(parse_subagent_tasks(&json!({"tasks": [{"nope": 1}, {"task": "real"}]})).len(), 1);
    }
