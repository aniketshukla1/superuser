use super::*;

    #[test]
    pub(super) fn a_window_that_has_gone_by_is_skipped_and_not_replayed() {
        let d = scratch();
        let mut store = AutomationStore::load(&d);
        let task = store.create_checked(a_task("morning brief", "0 9 * * *")).unwrap();
        // A laptop shut over the window: the firing is an hour late.
        let now = now();
        {
            let t = store.get_mut(&task.id).unwrap();
            t.next_run = Some(now - 3600.0);
        }
        assert!(!store.due(now).is_empty(), "before the sweep it looks due");
        let skipped = store.skip_missed(now);
        assert_eq!(skipped, vec![task.id.clone()]);
        assert!(store.due(now).is_empty(), "and after it, it is not — no backlog to replay");
        let after = store.get(&task.id).unwrap();
        assert_eq!(after.last_status.as_deref(), Some("skipped"));
        assert!(after.next_run.unwrap() > now, "the stamp moved forward");
        // A firing that is merely a little late still runs: the grace window is the point.
        {
            let t = store.get_mut(&task.id).unwrap();
            t.next_run = Some(now - 60.0);
        }
        assert!(store.skip_missed(now).is_empty());
        assert_eq!(store.due(now).len(), 1);
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    pub(super) fn a_run_the_app_died_in_the_middle_of_is_written_off() {
        let d = scratch();
        let mut store = AutomationStore::load(&d);
        let task = store.create_checked(a_task("nightly", "0 2 * * *")).unwrap();
        let run = store.start_run(&task.id, "schedule").unwrap();
        assert_eq!(store.reap_abandoned(now()), 0, "a run that just started is not abandoned");
        assert_eq!(store.reap_abandoned(now() + ABANDONED_SECS + 1.0), 1);
        let reaped = store.list_runs(&task.id).into_iter().find(|r| r.run_id == run.run_id).unwrap();
        assert_eq!(reaped.status, "skipped");
        assert_eq!(store.reap_abandoned(now() + ABANDONED_SECS + 2.0), 0, "written off once");
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    pub(super) fn it_says_so_the_first_time_it_breaks_and_switches_off_at_ten() {
        let d = scratch();
        let mut store = AutomationStore::load(&d);
        let task = store.create_checked(a_task("flaky", "0 9 * * *")).unwrap();
        let fail = |store: &mut AutomationStore| {
            let run = store.start_run(&task.id, "schedule").unwrap();
            let _ = store.finish_run(&task.id, &run.run_id, "error", None, vec![], Some("the API was down".into()));
        };
        fail(&mut store);
        let first = store.fatigue(&task.id, "the API was down").expect("one message, the first time");
        assert!(first.contains("failed") && first.contains("the API was down"));
        // The quiet middle: it fails every morning and says nothing more.
        for _ in 2..FATIGUE_LIMIT {
            fail(&mut store);
            assert_eq!(store.fatigue(&task.id, "the API was down"), None);
        }
        fail(&mut store);
        let last = store.fatigue(&task.id, "the API was down").expect("the tenth switches it off");
        assert!(last.contains("switched it off") && last.contains("turn it back on"));
        assert!(!store.get(&task.id).unwrap().enabled);
        assert!(store.get(&task.id).unwrap().next_run.is_none());
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    pub(super) fn a_skipped_run_is_not_a_failure_and_does_not_break_the_streak() {
        let d = scratch();
        let mut store = AutomationStore::load(&d);
        let task = store.create_checked(a_task("brief", "0 9 * * *")).unwrap();
        let run = store.start_run(&task.id, "schedule").unwrap();
        let _ = store.finish_run(&task.id, &run.run_id, "error", None, vec![], Some("down".into()));
        let missed = store.start_run(&task.id, "schedule").unwrap();
        let _ = store.finish_run(&task.id, &missed.run_id, "skipped", None, vec![], Some("missed its window".into()));
        assert_eq!(store.consecutive_failures(&task.id), 1, "a laptop that was shut says nothing about the work");
        let ok = store.start_run(&task.id, "schedule").unwrap();
        let _ = store.finish_run(&task.id, &ok.run_id, "ok", Some("done".into()), vec![], None);
        assert_eq!(store.consecutive_failures(&task.id), 0, "a success clears it");
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    pub(super) fn a_changed_automation_keeps_what_only_the_store_knows() {
        let d = scratch();
        let mut store = AutomationStore::load(&d);
        let mut made = a_task("brief", "0 9 * * *");
        made.origin_session = Some("s-123".into());
        let task = store.create_checked(made).unwrap();
        let run = store.start_run(&task.id, "schedule").unwrap();
        let _ = store.finish_run(&task.id, &run.run_id, "ok", Some("done".into()), vec![], None);

        let mut changed = store.get(&task.id).cloned().unwrap();
        changed.title = "morning brief".into();
        changed.schedule = daily("30 8 * * 1-5");
        let after = store.replace(&task.id, changed).unwrap();
        assert_eq!((after.title.as_str(), after.run_count), ("morning brief", 1));
        assert_eq!(after.origin_session.as_deref(), Some("s-123"), "it still answers where it was asked for");
        assert_eq!(after.created_at, task.created_at);
        assert!(after.next_run.is_some() && after.human_schedule().contains("weekday"));
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    pub(super) fn a_schedule_is_said_in_words_including_its_clock() {
        let words = |cron: &str, tz: &str| {
            Schedule { kind: "cron".into(), cron: Some(cron.into()), fire_at: None, timezone: tz.into() }.human()
        };
        assert_eq!(words("*/20 * * * *", "local"), "Every 20 minutes");
        assert_eq!(words("0 */3 * * *", "local"), "Every 3 hours, on the hour");
        assert_eq!(words("30 */6 * * *", "local"), "Every 6 hours at :30");
        assert_eq!(words("0 9 * * 1-5", "UTC"), "Every weekday at ~9:00 AM (UTC)");
        assert_eq!(words("0 9 * * *", "+02:00"), "Every day at ~9:00 AM (+02:00)");
        assert_eq!(words("0 9 1 * *", "local"), "Monthly on day 1 at ~9:00 AM");
        // A shape with no plain words for it still says what it is, rather than lying.
        assert_eq!(words("0 9 1-7 * 1", "local"), "0 9 1-7 * 1");
    }

    #[test]
    pub(super) fn a_cron_field_reads_every_shape_people_write() {
        // `*`, numbers, lists, ranges, steps, and steps over a range.
        assert!(field_matches("*", 7, false, (0, 23)));
        assert!(field_matches("*/20", 40, false, (0, 59)) && !field_matches("*/20", 41, false, (0, 59)));
        assert!(field_matches("9-17/2", 15, false, (0, 23)) && !field_matches("9-17/2", 16, false, (0, 23)));
        assert!(field_matches("1,3,5", 3, false, (0, 23)) && !field_matches("1,3,5", 4, false, (0, 23)));
        // `5/10` means "from 5 on, every 10" — which is what someone writing it expects.
        assert!(field_matches("5/10", 25, false, (0, 59)) && !field_matches("5/10", 20, false, (0, 59)));
        // Sunday is 0 and 7, and only day-of-week wraps.
        assert!(field_matches("7", 0, true, (0, 7)) && !field_matches("8", 1, false, (1, 31)));
        // Unreadable is not "matches everything".
        assert!(!field_matches("every-monday", 1, true, (0, 7)));
        assert!(!parses_field("every-monday") && parses_field("*/5") && parses_field("1-5"));
        assert!(cron_fields("0 9 * * 1-5").is_some());
        assert!(cron_fields("0 9 * *").is_none(), "four fields is not cron");
        assert!(cron_fields("0 9 * * everyday").is_none());
    }

    #[test]
    pub(super) fn a_step_schedule_fires_on_its_step() {
        use chrono::{Local, TimeZone, Timelike};
        let every_20 = daily("*/20 * * * *");
        let first = next_run_after(&every_20, Local.with_ymd_and_hms(2026, 3, 2, 9, 5, 0).unwrap().timestamp() as f64).unwrap();
        let at = Local.timestamp_opt(first as i64, 0).unwrap();
        assert_eq!((at.hour(), at.minute()), (9, 20));
        let then = next_run_after(&every_20, first).unwrap();
        assert_eq!(then - first, 20.0 * 60.0);
        // The month field is read too — one that never comes round is no next run.
        let never = Schedule { kind: "cron".into(), cron: Some("0 9 30 2 *".into()), fire_at: None, timezone: "local".into() };
        assert_eq!(next_run_after(&never, 0.0), None, "30 February never arrives");
    }

    #[test]
    pub(super) fn a_schedule_keeps_its_own_clock() {
        use chrono::{TimeZone, Timelike, Utc};
        let utc = Schedule { kind: "cron".into(), cron: Some("0 9 * * *".into()), fire_at: None, timezone: "UTC".into() };
        let ts = next_run_after(&utc, Utc.with_ymd_and_hms(2026, 3, 2, 3, 0, 0).unwrap().timestamp() as f64).unwrap();
        assert_eq!(Utc.timestamp_opt(ts as i64, 0).unwrap().hour(), 9);
        // An offset is honoured: 09:00 at +02:00 is 07:00 UTC.
        let madrid = Schedule { kind: "cron".into(), cron: Some("0 9 * * *".into()), fire_at: None, timezone: "+02:00".into() };
        let ts = next_run_after(&madrid, Utc.with_ymd_and_hms(2026, 3, 2, 3, 0, 0).unwrap().timestamp() as f64).unwrap();
        assert_eq!(Utc.timestamp_opt(ts as i64, 0).unwrap().hour(), 7);
        // A name the zone database doesn't have is refused rather than guessed at; one it has
        // is kept by name (see `a_named_zone_keeps_nine_oclock_at_nine_across_daylight_saving`).
        assert_eq!(zone_of("Europe/Nowhere"), None);
        assert_eq!(zone_of("Europe/Madrid"), Some(Zone::Named("Europe/Madrid".into())));
        assert_eq!(zone_of(""), Some(Zone::Local));
        assert_eq!(zone_of("utc"), Some(Zone::Fixed(0)));
        assert_eq!(zone_of("-0500"), Some(Zone::Fixed(-5 * 3600)));
    }

    #[test]
    pub(super) fn a_schedule_that_fires_too_often_is_refused_whenever_it_is_asked_about() {
        assert!(schedule_problem(&daily("* * * * *")).unwrap().contains("more often"));
        assert!(schedule_problem(&daily("*/5 * * * *")).unwrap().contains("more often"));
        assert_eq!(schedule_problem(&daily("*/15 * * * *")), None, "every fifteen minutes is the floor, not past it");
        assert_eq!(schedule_problem(&daily("0 9 * * 1-5")), None);
        // The refusal is about the schedule, not about the clock it was asked on: a
        // deterministic scan from a fixed date is half of why.
        assert!(schedule_problem(&daily("0,10 9 * * *")).unwrap().contains("more often"));
        // Unreadable expressions and zones say what is wrong with them.
        assert!(schedule_problem(&daily("0 9 * *")).unwrap().contains("cron expression"));
        let zoned = Schedule { kind: "cron".into(), cron: Some("0 9 * * *".into()), fire_at: None, timezone: "Mars/Olympus_Mons".into() };
        assert!(schedule_problem(&zoned).unwrap().contains("isn't a timezone this machine knows"));
    }
