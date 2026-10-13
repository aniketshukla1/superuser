use super::*;
    use serde_json::json;

    fn req() -> Request {
        Request {
            user_said: "run the tests".into(),
            tool: "run_shell".into(),
            args: json!({ "command": "cargo test" }),
            workspace: "/repo".into(),
        }
    }

    #[test]
    fn the_three_verdicts_are_read() {
        assert!(matches!(parse(r#"{"verdict":"allow","reason":"exactly what you asked"}"#), Verdict::Allow { .. }));
        assert!(matches!(parse(r#"{"verdict":"deny","reason":"nope"}"#), Verdict::Deny { .. }));
        assert!(matches!(parse(r#"{"verdict":"unsure","reason":"cannot tell"}"#), Verdict::Unsure { .. }));
    }

    #[test]
    fn wrapping_is_tolerated_because_models_wrap() {
        let fenced = "Sure!\n```json\n{\"verdict\": \"allow\", \"reason\": \"fine\"}\n```\n";
        assert!(matches!(parse(fenced), Verdict::Allow { .. }));
    }

    #[test]
    fn every_malformed_answer_lands_on_the_human() {
        // The whole safety property in one test: there is no input to `parse` that
        // produces an Allow by accident.
        for bad in [
            "",
            "   ",
            "yes, go ahead",
            "{",
            "{\"verdict\":}",
            r#"{"reason":"no verdict field"}"#,
            r#"{"verdict":"approve","reason":"invented word"}"#,
            r#"{"verdict":"ALLOW IT","reason":"close enough"}"#,
            r#"{"verdict":true}"#,
        ] {
            assert!(
                matches!(parse(bad), Verdict::Unsure { .. }),
                "must fail closed, got something else for: {bad:?}"
            );
        }
    }

    #[test]
    fn a_verdict_without_a_reason_still_gets_one() {
        // The reason is shown to the user on the card, so an empty one is a card
        // that says nothing.
        match parse(r#"{"verdict":"deny"}"#) {
            Verdict::Deny { reason } => assert!(!reason.is_empty()),
            other => panic!("expected deny, got {other:?}"),
        }
    }

    #[test]
    fn the_judge_is_never_handed_anything_a_stranger_wrote() {
        // Invariant 4, checked on the actual prompt: what goes up is the user's
        // words, the tool, its arguments, and the folder NAME. If a field for tool
        // output is ever added to `Request`, this test is where it should hurt.
        let block = action_block(&req());
        assert!(block.contains("run the tests"));
        assert!(block.contains("cargo test"));
        assert!(block.contains("/repo"));
        // Four labelled sections and nothing else — no transcript, no file bodies.
        assert_eq!(block.matches("Working folder:").count(), 1);
        assert_eq!(block.matches("The user asked:").count(), 1);
        assert_eq!(block.matches("Proposed action:").count(), 1);
    }

    #[test]
    fn an_enormous_argument_cannot_crowd_out_the_question() {
        let mut r = req();
        r.args = json!({ "content": "x".repeat(50_000) });
        assert!(action_block(&r).len() < 3_000);
    }

    #[test]
    fn a_turn_with_no_user_words_says_so_rather_than_looking_empty() {
        let mut r = req();
        r.user_said = "  ".into();
        assert!(action_block(&r).contains("(nothing this turn)"));
    }

    fn deny() -> Verdict {
        Verdict::Deny { reason: "no".into() }
    }

    #[test]
    fn five_denials_in_a_row_trip_the_breaker_and_the_fifth_says_so() {
        let b = Breaker::default();
        for i in 1..TRIP {
            assert!(!b.record(&deny()), "tripped early at {i}");
            assert!(!b.is_paused());
        }
        assert!(b.record(&deny()), "the fifth is the one that trips it");
        assert!(b.is_paused());
    }

    #[test]
    fn the_trip_is_announced_exactly_once() {
        // Otherwise every subsequent card in the turn repeats the notice.
        let b = Breaker::default();
        for _ in 0..TRIP {
            b.record(&deny());
        }
        for _ in 0..3 {
            assert!(!b.record(&deny()), "the notice must not fire again");
        }
        assert!(b.is_paused());
    }

    #[test]
    fn any_non_deny_resets_the_streak() {
        // The reason this is a streak and not a total: an over-strict pair early in
        // a long turn must not silently downgrade the rest of it.
        let b = Breaker::default();
        b.record(&deny());
        b.record(&deny());
        b.record(&deny());
        b.record(&deny());
        assert_eq!(b.denials(), 4);
        b.record(&Verdict::Allow { reason: "fine".into() });
        assert_eq!(b.denials(), 0);
        for _ in 0..4 {
            assert!(!b.record(&deny()), "the streak restarted, so four more must not trip it");
        }
        assert!(!b.is_paused());
    }

    #[test]
    fn an_abstention_is_not_a_denial() {
        // A merely cautious model answers `unsure` a lot. Folding that into the
        // streak would trip the breaker on a reviewer that is working correctly.
        let b = Breaker::default();
        for _ in 0..(TRIP * 3) {
            assert!(!b.record(&Verdict::unsure("cannot tell")));
        }
        assert!(!b.is_paused());
        assert_eq!(b.denials(), 0);
    }

    #[test]
    fn a_tripped_breaker_stays_tripped_for_the_turn() {
        let b = Breaker::default();
        for _ in 0..TRIP {
            b.record(&deny());
        }
        b.record(&Verdict::Allow { reason: "fine".into() });
        assert!(b.is_paused(), "an allow resets the streak but must not un-pause the turn");
    }

    #[test]
    fn the_agent_is_told_nothing_it_could_route_around() {
        // The user gets the reviewer's actual reason; the agent gets this. It must
        // not name what was objected to, or the next attempt is built to avoid it.
        assert!(!AGENT_DENY_MESSAGE.contains("because"));
        assert!(AGENT_DENY_MESSAGE.contains("ask the user"));
    }
