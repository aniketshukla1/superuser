use super::*;

    #[test]
    fn globs_match_the_way_a_user_would_expect() {
        assert!(glob_match("notion_*", "notion_search_pages"));
        assert!(glob_match("*_delete_*", "github_delete_repo"));
        assert!(glob_match("exact_name", "exact_name"));
        assert!(glob_match("*", "anything"));
        assert!(glob_match("a?c", "abc"));
        assert!(!glob_match("a?c", "abbc"));
        assert!(!glob_match("notion_*", "linear_search"));
        assert!(!glob_match("exact_name", "exact_name_2"));
    }

    #[test]
    fn a_literal_dot_stays_a_literal_dot() {
        // The reason this is not a regex translation: a missed escape silently
        // widens the rule the user wrote.
        assert!(glob_match("app.read", "app.read"));
        assert!(!glob_match("app.read", "appxread"));
    }

    #[test]
    fn the_most_specific_rule_wins() {
        let o = RiskOverrides::from_pairs(&[
            ("notion_*", RiskClass::Read),
            ("notion_delete_*", RiskClass::External),
        ]);
        assert_eq!(o.effective("notion_search_pages", RiskClass::External), RiskClass::Read);
        assert_eq!(o.effective("notion_delete_page", RiskClass::Read), RiskClass::External);
    }

    #[test]
    fn an_exact_pattern_beats_any_glob() {
        let o = RiskOverrides::from_pairs(&[
            ("*", RiskClass::Read),
            ("linear_create_issue", RiskClass::External),
        ]);
        assert_eq!(o.effective("linear_create_issue", RiskClass::Read), RiskClass::External);
        assert_eq!(o.effective("linear_list_issues", RiskClass::External), RiskClass::Read);
    }

    #[test]
    fn a_built_in_can_be_tightened_but_never_loosened() {
        // The line of JSON that would otherwise switch off the command gate, the
        // mode check and the approval card at once.
        let loosen = RiskOverrides::from_pairs(&[("run_shell", RiskClass::Read), ("*", RiskClass::Read)]);
        assert_eq!(loosen.effective("run_shell", RiskClass::Exec), RiskClass::Exec);
        assert_eq!(loosen.effective("send_message", RiskClass::External), RiskClass::External);
        assert_eq!(loosen.effective("write_file", RiskClass::WriteLocal), RiskClass::WriteLocal);

        // Tightening is always allowed — the user is entitled to be stricter than us.
        let tighten = RiskOverrides::from_pairs(&[("write_file", RiskClass::External)]);
        assert_eq!(tighten.effective("write_file", RiskClass::WriteLocal), RiskClass::External);
    }

    #[test]
    fn a_wildcard_cannot_launder_a_built_in() {
        // Naming it directly is refused above; naming it by pattern must be too.
        let o = RiskOverrides::from_pairs(&[("run_*", RiskClass::Read)]);
        assert_eq!(o.effective("run_shell", RiskClass::Exec), RiskClass::Exec);
    }

    #[test]
    fn an_mcp_tool_may_be_relaxed_because_its_class_was_only_ever_a_guess() {
        let o = RiskOverrides::from_pairs(&[("myserver_run_report", RiskClass::Read)]);
        assert_eq!(o.effective("myserver_run_report", RiskClass::External), RiskClass::Read);
    }

    #[test]
    fn the_card_is_told_which_rules_can_never_bite() {
        // Silently dropping a rule is how someone concludes the feature is broken.
        let o = RiskOverrides::from_pairs(&[
            ("run_shell", RiskClass::Read),
            ("notion_*", RiskClass::Read),
            ("write_file", RiskClass::External),
        ]);
        let view = o.view();
        let by = |p: &str| view.iter().find(|v| v.pattern == p).unwrap().clone();
        assert!(!by("run_shell").effective, "loosening a built-in cannot bite");
        assert!(by("notion_*").effective, "an MCP tool can be relaxed");
        assert!(by("write_file").effective, "tightening a built-in is allowed");
    }

    #[test]
    fn a_glob_that_reaches_a_builtin_is_flagged_too() {
        let o = RiskOverrides::from_pairs(&[("*", RiskClass::Read)]);
        assert!(!o.view()[0].effective, "a wildcard reaches run_shell without naming it");
    }

    #[test]
    fn saving_rejects_a_risk_class_that_does_not_exist() {
        let dir = std::env::temp_dir().join(format!("ovr-save-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert!(RiskOverrides::save(&dir, &[("a_*".into(), "banana".into())]).is_err());
        assert!(RiskOverrides::save(&dir, &[("  ".into(), "read".into())]).is_err());

        let saved = RiskOverrides::save(&dir, &[("a_*".into(), "read".into())]).unwrap();
        assert_eq!(saved.effective("a_thing", RiskClass::External), RiskClass::Read);
        // And it round-trips through the file, not just the returned value.
        assert_eq!(RiskOverrides::load(&dir).view().len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn no_rules_changes_nothing() {
        let o = RiskOverrides::default();
        assert_eq!(o.effective("anything", RiskClass::External), RiskClass::External);
        assert_eq!(o.effective("run_shell", RiskClass::Exec), RiskClass::Exec);
    }

    #[test]
    fn a_broken_file_means_no_overrides_rather_than_no_tools() {
        let dir = std::env::temp_dir().join(format!("ovr-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("risk-overrides.json"), "{ not json at all").unwrap();
        assert_eq!(RiskOverrides::load(&dir).effective("x_y", RiskClass::External), RiskClass::External);

        // An unknown risk word drops that rule and keeps the rest.
        std::fs::write(
            dir.join("risk-overrides.json"),
            r#"[{"pattern":"a_*","risk":"banana"},{"pattern":"b_*","risk":"read"}]"#,
        )
        .unwrap();
        let o = RiskOverrides::load(&dir);
        assert_eq!(o.effective("a_thing", RiskClass::External), RiskClass::External);
        assert_eq!(o.effective("b_thing", RiskClass::External), RiskClass::Read);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_missing_file_is_the_normal_case() {
        assert_eq!(
            RiskOverrides::load(Path::new("/nonexistent")).effective("x", RiskClass::External),
            RiskClass::External
        );
    }
