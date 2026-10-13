use super::*;

    #[test]
    fn clicks_either_side_of_a_grid_line_are_still_the_same_click() {
        // The case the bucketing version got wrong: 298 and 300 straddle a 25px line.
        let s: Vec<String> = ["left_click at (400, 298)", "left_click at (400, 300)", "left_click at (401, 302)"]
            .iter()
            .map(|d| cu_signature(d))
            .collect();
        assert!(cu_stalled(&s), "{s:?}");
    }

    #[test]
    fn a_different_kind_of_action_is_not_the_same_action() {
        let s: Vec<String> = ["left_click at (400, 300)", "double_click at (400, 300)", "left_click at (400, 300)"]
            .iter()
            .map(|d| cu_signature(d))
            .collect();
        assert!(!cu_stalled(&s));
    }

    #[test]
    fn three_near_identical_clicks_are_a_stall() {
        let s: Vec<String> = ["left_click at (412, 300)", "left_click at (409, 302)", "left_click at (414, 298)"]
            .iter()
            .map(|d| cu_signature(d))
            .collect();
        assert!(cu_stalled(&s), "clicks a few pixels apart are the same click: {s:?}");
    }

    #[test]
    fn two_is_a_retry_and_different_targets_are_progress() {
        let two: Vec<String> = ["left_click at (412, 300)", "left_click at (412, 300)"].iter().map(|d| cu_signature(d)).collect();
        assert!(!cu_stalled(&two), "a second click on a slow button is a reasonable retry");
        let moving: Vec<String> = ["left_click at (100, 100)", "left_click at (400, 100)", "left_click at (100, 400)"]
            .iter()
            .map(|d| cu_signature(d))
            .collect();
        assert!(!cu_stalled(&moving));
    }

    #[test]
    fn a_later_plan_step_can_use_an_earlier_ones_output() {
        let args = serde_json::json!({"pattern": "{{0}}", "nested": {"x": ["{{1}}"]}});
        let out = plan_substitute(&args, &["needle\n".into(), "second".into()]);
        assert_eq!(out["pattern"], "needle");
        assert_eq!(out["nested"]["x"][0], "second");
        assert_eq!(plan_substitute(&serde_json::Value::Null, &[]), serde_json::json!({}));
    }
