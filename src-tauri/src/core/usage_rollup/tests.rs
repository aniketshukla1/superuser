use super::*;
    use crate::core::sessions::{Message, Session, TokenUsage};

    fn at(secs: f64, usage: Option<(u64, u64)>) -> Message {
        let mut m = Message::new("assistant", "hi");
        m.at = secs;
        m.usage = usage.map(|(i, o)| TokenUsage { input: i, output: o, ..Default::default() });
        m
    }

    fn session(msgs: Vec<Message>) -> Session {
        let mut s = Session::new("cowork", "", "m");
        s.messages = msgs;
        s
    }

    /// Noon local on a Wednesday, so day and week boundaries are both interesting and
    /// neither sits on the value under test.
    fn wednesday_noon() -> f64 {
        let d = Local
            .with_ymd_and_hms(2026, 8, 26, 12, 0, 0)
            .earliest()
            .expect("2026-08-26 12:00 local exists");
        assert_eq!(d.weekday().num_days_from_monday(), 2, "fixture must be a Wednesday");
        d.timestamp() as f64
    }

    #[test]
    fn today_is_since_local_midnight_and_the_week_is_since_monday() {
        let now = wednesday_noon();
        let s = session(vec![
            at(now - 60.0, Some((10, 5))),            // an hour ago: today + week
            at(now - 20.0 * 3600.0, Some((100, 50))), // yesterday: week only
            at(now - 9.0 * 86_400.0, Some((7, 7))),   // last week: neither
        ]);
        let r = rollup_at(&[s], now);
        assert_eq!(r.today.tokens, 15);
        assert_eq!(r.today.counted, 1);
        assert_eq!(r.week.tokens, 165);
        assert_eq!(r.week.counted, 2);
    }

    #[test]
    fn a_turn_that_reported_nothing_is_counted_as_unknown_not_as_zero() {
        let now = wednesday_noon();
        let s = session(vec![at(now - 60.0, Some((10, 5))), at(now - 61.0, None)]);
        let r = rollup_at(&[s], now);
        assert_eq!(r.today.tokens, 15);
        assert_eq!(r.today.counted, 1);
        assert_eq!(r.today.uncounted, 1, "the silent turn must be visible, not folded into the total");
        assert!(!r.today.complete(), "a window with unknown turns is a floor, not a total");
    }

    #[test]
    fn user_turns_are_not_added_on_top_of_the_reply_that_already_billed_them() {
        let now = wednesday_noon();
        let mut u = Message::new("user", "hello");
        u.at = now - 60.0;
        u.usage = Some(TokenUsage { input: 999, output: 999, ..Default::default() });
        let r = rollup_at(&[session(vec![u, at(now - 59.0, Some((10, 5)))])], now);
        assert_eq!(r.today.tokens, 15);
        assert_eq!(r.today.counted, 1);
        assert_eq!(r.today.uncounted, 0);
    }

    #[test]
    fn the_windows_roll_over_ahead_of_now_and_the_week_lands_on_a_monday() {
        let now = wednesday_noon();
        let r = rollup_at(&[], now);
        assert!(r.today.resets_at > now);
        assert!(r.week.resets_at > r.today.resets_at);
        let wk = Local.timestamp_opt(r.week.resets_at as i64, 0).earliest().unwrap();
        assert_eq!(wk.weekday().num_days_from_monday(), 0, "the week must roll over on a Monday");
        assert_eq!((wk.hour(), wk.minute()), (0, 0));
    }

    #[test]
    fn clock_skew_and_undated_messages_are_skipped_rather_than_counted_somewhere_wrong() {
        let now = wednesday_noon();
        let s = session(vec![
            at(0.0, Some((5, 5))),          // never stamped
            at(now + 600.0, Some((5, 5))),  // ten minutes into the future
            at(now - 60.0, Some((1, 1))),
        ]);
        let r = rollup_at(&[s], now);
        assert_eq!(r.today.tokens, 2);
        assert_eq!(r.today.counted, 1);
        assert_eq!(r.today.uncounted, 0);
    }

    #[test]
    fn an_empty_history_is_zero_rather_than_an_error() {
        let r = rollup_at(&[], wednesday_noon());
        assert_eq!(r.today.tokens, 0);
        assert!(r.today.complete(), "nothing unknown when there is nothing at all");
    }
