use super::*;

    #[test]
    pub(super) fn a_firing_is_told_that_it_is_the_firing() {
        let framed_text = format!("{FIRING_FRAME}\n\nAppend today's total to the sheet.");
        assert!(FIRING_FRAME.contains("going off on its schedule") && FIRING_FRAME.contains("Leave automations alone"));
        assert_eq!(without_frame(&framed_text), "Append today's total to the sheet.");
        assert_eq!(without_frame("Append today's total to the sheet."), "Append today's total to the sheet.");
        // A manual run carries the same instruction, without claiming a schedule it isn't on.
        let manual = framed("Append today's total to the sheet.", true);
        assert!(manual.starts_with(MANUAL_FRAME) && !MANUAL_FRAME.contains("on its schedule"));
        assert!(MANUAL_FRAME.ends_with(FIRING_FRAME.split_once(". ").unwrap().1), "the instruction half must not drift apart");
        assert_eq!(without_frame(&manual), "Append today's total to the sheet.");
        assert_eq!(framed("x", false), format!("{FIRING_FRAME}\n\nx"));
    }

    #[test]
    pub(super) fn weekday_ranges_and_lists_are_understood() {
        use chrono::{Datelike, Local, TimeZone, Timelike};
        // `0 9 * * 1-5` is the canonical "weekdays at 9am" — the form every example
        // uses, and the one the schedule tool's own description suggests.
        let next = next_run_after(&daily("0 9 * * 1-5"), 0.0).expect("weekday range must parse");
        let d = Local.timestamp_opt(next as i64, 0).single().unwrap();
        assert!((1..=5).contains(&d.weekday().num_days_from_sunday()), "landed on {}", d.weekday());
        assert_eq!(d.hour(), 9);

        // A list, and Sunday written both ways.
        assert!(next_run_after(&daily("0 9 * * 1,3,5"), 0.0).is_some());
        let sun0 = next_run_after(&daily("0 9 * * 0"), 0.0).unwrap();
        let sun7 = next_run_after(&daily("0 9 * * 7"), 0.0).unwrap();
        assert_eq!(sun0, sun7, "0 and 7 both mean Sunday");
        assert_eq!(Local.timestamp_opt(sun0 as i64, 0).single().unwrap().weekday(), chrono::Weekday::Sun);
    }

    #[test]
    pub(super) fn a_day_of_month_never_wraps_to_a_weekday() {
        use chrono::{Datelike, Local, TimeZone};
        // Mod-7 matching would make "the 1st" also fire on the 8th, 15th and 22nd.
        let next = next_run_after(&daily("0 9 1 * *"), 0.0).unwrap();
        assert_eq!(Local.timestamp_opt(next as i64, 0).single().unwrap().day(), 1);
    }

    #[test]
    pub(super) fn an_unparseable_schedule_is_rejected_rather_than_meaning_never() {
        // Each of these used to walk 366 days matching nothing and return None,
        // which reads identically to "valid but nothing due soon".
        for bad in ["0 9 * * MON", "0 9 * * mon-fri", "0 9 * *", "60 9 * * *", "0 24 * * *", "0 9 * * "] {
            assert!(next_run_after(&daily(bad), 0.0).is_none(), "{bad} should be refused");
        }
        // `*/5` moved out of this list when steps became readable: it is a real schedule
        // that computes a real next run, and what is wrong with it is how OFTEN it fires.
        // That is the floor's judgement, not the parser's.
        assert!(next_run_after(&daily("*/5 * * * *"), 0.0).is_some());
        assert!(schedule_problem(&daily("*/5 * * * *")).is_some());
    }

    #[test]
    pub(super) fn a_one_off_in_the_past_is_not_a_next_run() {
        let past = Schedule { kind: "once".into(), cron: None, fire_at: Some("2023-11-14T09:00".into()), timezone: "local".into() };
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs_f64();
        // Against the epoch it looks fine — which is exactly the bug: a model with no
        // date in its context proposed a 2023 timestamp and it was accepted.
        assert!(next_run_after(&past, 0.0).is_some());
        assert!(next_run_after(&past, now).is_none(), "must be refused against the real clock");
    }

    #[test]
    pub(super) fn schedule_human_labels() {
        assert_eq!(daily("10 19 * * *").human(), "Every day at ~7:10 PM");
        assert_eq!(daily("0 9 * * 1").human(), "Every Monday at ~9:00 AM"); // standard cron: 1 = Monday
        assert_eq!(daily("0 9 * * 0").human(), "Every Sunday at ~9:00 AM");
        assert_eq!(daily("0 9 * * 1-5").human(), "Every weekday at ~9:00 AM");
        assert_eq!(daily("0 9 * * 1,3").human(), "Every Monday, Wednesday at ~9:00 AM");
        assert_eq!(daily("30 8 15 * *").human(), "Monthly on day 15 at ~8:30 AM");
        let once = Schedule { kind: "once".into(), cron: None, fire_at: Some("2026-09-01T09:00".into()), timezone: "local".into() };
        assert!(once.human().starts_with("Once at"));
    }

    #[test]
    pub(super) fn standing_rules_roundtrip() {
        let mut t = ScheduledTask::new("t".into(), "do".into(), daily("0 7 * * *"), "/tmp".into(), None, None);
        assert!(t.add_rule("send_slack", "#ops"));
        assert!(!t.add_rule("send_slack", "#ops")); // dup
        let rules = t.standing_rules();
        assert_eq!(rules["send_slack"], vec!["#ops"]);
        assert!(t.revoke_rule("send_slack #ops"));
        assert!(t.standing_rules().is_empty());
    }

    #[test]
    pub(super) fn next_run_daily_at_the_right_time() {
        use chrono::{Local, TimeZone, Timelike};
        let now = now();
        let n = next_run_after(&daily("30 9 * * *"), now).unwrap();
        assert!(n > now && n - now <= 24.0 * 3600.0 + 60.0);
        let dt = Local.timestamp_opt(n as i64, 0).single().unwrap();
        assert_eq!((dt.hour(), dt.minute()), (9, 30));
    }

    #[test]
    pub(super) fn next_run_weekly_lands_on_weekday() {
        use chrono::{Datelike, Local, TimeZone};
        let now = now();
        // Standard cron: 0 = Sunday, so dow 2 is Tuesday. This used to be Monday-based,
        // which made `0 9 * * 1-5` mean Tue–Sat rather than the weekdays everyone
        // (and every cron reference) means by it.
        let n = next_run_after(&daily("0 8 * * 2"), now).unwrap();
        assert!(n > now && n - now <= 7.0 * 24.0 * 3600.0 + 60.0);
        let dt = Local.timestamp_opt(n as i64, 0).single().unwrap();
        assert_eq!(dt.weekday(), chrono::Weekday::Tue);
    }

    #[test]
    pub(super) fn once_in_the_past_never_fires() {
        let s = Schedule { kind: "once".into(), cron: None, fire_at: Some("2000-01-01T09:00".into()), timezone: "local".into() };
        assert!(next_run_after(&s, now()).is_none());
    }

    #[test]
    pub(super) fn store_due_and_advance() {
        let dir = scratch();
        let mut store = AutomationStore::load(&dir);

        // A task already past-due (next_run in the past) is returned by due().
        let mut overdue = ScheduledTask::new("nightly".into(), "run".into(), daily("0 0 * * *"), "/tmp".into(), None, None);
        overdue.next_run = Some(now() - 100.0);
        let overdue = store.create(overdue);
        assert!(store.due(now()).iter().any(|t| t.id == overdue.id));

        // A freshly-created task computes a future next_run → not due.
        let future = store.create(ScheduledTask::new("morning".into(), "brief".into(), daily("0 7 * * *"), "/tmp".into(), None, None));
        assert!(future.next_run.map(|n| n > now()).unwrap_or(false));
        assert!(!store.due(now()).iter().any(|t| t.id == future.id));

        // advance() rolls the overdue task forward past now.
        store.advance(&overdue.id, now()).unwrap();
        assert!(store.get(&overdue.id).unwrap().next_run.unwrap() > now());
        assert!(!store.due(now()).iter().any(|t| t.id == overdue.id));

        let _ = fs::remove_dir_all(&dir);
    }

    /// The unread mark, which nothing had ever computed.
    ///
    /// `unseen_runs` and `unseen_failed` were in the GUI's type and read by its
    /// cards; the server sent neither, so every automation looked equally read
    /// whether it had run overnight or not.
    #[test]
    pub(super) fn unseen_counts_finished_runs_since_you_last_looked() {
        let dir = scratch();
        let mut store = AutomationStore::load(&dir);
        let id = store
            .create(ScheduledTask::new("Brief".into(), "x".into(), daily("0 7 * * *"), "/tmp".into(), None, None))
            .id;

        assert_eq!(store.unseen(&id), (0, false), "a task that never ran has no news");

        let a = store.start_run(&id, "schedule").unwrap();
        // Still going is not news yet — it is a thing in progress, not a result.
        assert_eq!(store.unseen(&id).0, 0);
        store.finish_run(&id, &a.run_id, "ok", Some("done".into()), vec![], None).unwrap();
        assert_eq!(store.unseen(&id), (1, false));

        let b = store.start_run(&id, "schedule").unwrap();
        store.finish_run(&id, &b.run_id, "error", None, vec![], Some("boom".into())).unwrap();
        // Two unread, and the NEWEST one failed — which is what tints the card.
        assert_eq!(store.unseen(&id), (2, true));

        store.mark_seen(&id).unwrap();
        assert_eq!(store.unseen(&id), (0, false), "opening the detail page clears it");

        // …and it stays cleared across a reload, since the mark is on the task.
        assert_eq!(AutomationStore::load(&dir).unseen(&id), (0, false));
        let _ = fs::remove_dir_all(&dir);
    }
