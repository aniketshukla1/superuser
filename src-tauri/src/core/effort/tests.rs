use super::*;

    /// Every model id this app can actually send to Anthropic.
    const SHIPPED: &[&str] = &[
        "claude-opus-5", "claude-sonnet-5", "claude-fable-5", "claude-opus-4-8",
        "claude-opus-4-7", "claude-haiku-4-5-20251001",
    ];

    #[test]
    fn no_current_model_is_ever_sent_a_token_budget() {
        // The P0. `budget_tokens` is a 400 on 4.7 and later, and this app sent it to
        // every Claude model — so the effort slider failed the turn on every current
        // one the moment it left the left stop. The default hid it, because `low`
        // sends no thinking field at all.
        for model in SHIPPED {
            for level in LEVELS {
                let (thinking, _) = request_fields(model, level, 32000);
                let Some(t) = thinking else { continue };
                if !accepts_budget(model) {
                    assert!(t.get("budget_tokens").is_none(), "{model} at {level} would 400: {t}");
                    assert_eq!(t["type"], "adaptive", "{model} at {level}");
                }
            }
        }
    }

    #[test]
    fn a_model_that_only_takes_the_old_mode_still_gets_it() {
        // The reverse trap: `type: adaptive` is a 400 on 4.5 and earlier, so this
        // could not be a blanket switch. Haiku 4.5 is the one we ship.
        assert!(!accepts_adaptive("claude-haiku-4-5-20251001"));
        let (thinking, effort) = request_fields("claude-haiku-4-5-20251001", "high", 32000);
        let t = thinking.expect("haiku 4.5 still thinks");
        assert_eq!(t["type"], "enabled");
        assert!(t["budget_tokens"].as_u64().unwrap() >= 1024);
        assert_eq!(effort, None, "haiku 4.5 has no effort parameter");
    }

    #[test]
    fn the_slider_maps_onto_the_api_scale_one_for_one_where_it_can() {
        for level in LEVELS {
            assert_eq!(effort_for("claude-opus-5", level), Some(level));
        }
    }

    #[test]
    fn an_unsupported_level_steps_down_rather_than_up() {
        // `xhigh` is newer than `max`, so some models have `max` and not `xhigh`.
        // Stepping UP would charge someone for `max` when they asked for `xhigh`.
        assert_eq!(effort_for("claude-sonnet-4-6", "xhigh"), Some("high"));
        assert_eq!(effort_for("claude-sonnet-4-6", "max"), Some("max"));
        assert_eq!(effort_for("claude-opus-4-5-20251101", "max"), Some("high"));
        assert_eq!(effort_for("claude-opus-4-5-20251101", "xhigh"), Some("high"));
        assert_eq!(effort_for("claude-haiku-4-5-20251001", "high"), None);
    }

    #[test]
    fn a_provider_prefix_and_a_date_suffix_both_resolve_to_the_family() {
        assert!(accepts_adaptive("anthropic:claude-opus-5"));
        assert!(!accepts_budget("anthropic:claude-opus-5"));
        assert_eq!(effort_for("anthropic:claude-opus-5", "max"), Some("max"));
        assert!(!accepts_adaptive("claude-opus-4-5-20251101"));
        assert!(accepts_budget("claude-opus-4-5-20251101"));
    }

    #[test]
    fn an_unknown_model_is_sent_nothing_it_might_reject() {
        let (thinking, effort) = request_fields("gemma4:12b", "high", 32000);
        assert_eq!(effort, None);
        assert!(thinking.is_some(), "unknown models keep today's behaviour");
    }

    #[test]
    fn the_scale_runs_faster_to_smarter_and_survives_a_round_trip() {
        for (i, l) in LEVELS.iter().enumerate() {
            assert_eq!(index_of(l), i);
            assert_eq!(at(i), *l);
        }
        // Budgets only ever go up as you move right.
        let budgets: Vec<u32> = LEVELS.iter().map(|l| budget(l, 32000).unwrap_or(0)).collect();
        let mut sorted = budgets.clone();
        sorted.sort_unstable();
        assert_eq!(budgets, sorted, "effort must not get cheaper as it gets smarter");
    }

    #[test]
    fn junk_and_older_configs_land_on_the_default_rather_than_breaking() {
        for s in ["", "  ", "ultra", "HIGH", "Max ", "reasonable"] {
            assert!(is_level(normalize(s)), "{s:?} normalised to a non-level");
        }
        assert_eq!(normalize("HIGH"), "high");
        assert_eq!(normalize("Max "), "max");
        assert_eq!(normalize("ultra"), DEFAULT);
    }

    #[test]
    fn the_budget_always_leaves_room_for_the_reply() {
        for max in [1025u32, 1500, 2000, 4096, 8192, 32000, 64000] {
            for l in LEVELS {
                if let Some(b) = budget(l, max) {
                    assert!(b >= MIN_BUDGET, "{l} at max_tokens={max} asked for {b}");
                    assert!(b < max, "{l} at max_tokens={max} left no room for the reply");
                }
            }
        }
    }

    #[test]
    fn a_reply_cap_too_small_to_think_in_asks_for_no_thinking_rather_than_a_400() {
        // 1024 is the floor AND must be strictly under max_tokens, so anything at or
        // below it has nowhere to put a thinking block.
        for max in [0u32, 1, 512, 1023, 1024] {
            for l in LEVELS {
                assert_eq!(budget(l, max), None, "{l} at max_tokens={max}");
                assert!(thinking_field(l, max).is_none());
            }
        }
    }

    #[test]
    fn low_means_no_thinking_on_either_vendor() {
        assert_eq!(budget("low", 32000), None);
        assert_eq!(reasoning_effort("low"), None);
    }

    #[test]
    fn nothing_above_high_is_sent_to_a_vendor_that_has_never_heard_of_it() {
        for l in ["xhigh", "max"] {
            assert_eq!(reasoning_effort(l), Some("high"));
        }
    }

    #[test]
    fn the_thinking_field_is_shaped_the_way_the_api_wants_it() {
        let v = thinking_field("max", 32000).expect("max should think");
        assert_eq!(v["type"], "enabled");
        assert_eq!(v["budget_tokens"], 28800);
    }
