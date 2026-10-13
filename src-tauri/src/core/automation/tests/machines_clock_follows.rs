use super::*;

    pub(super) fn scratch() -> PathBuf {
        static SCRATCH_N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let seq = SCRATCH_N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let n = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let dir = std::env::temp_dir().join(format!("bba-automation-{}-{}-{}", std::process::id(), n, seq));
        let _ = fs::create_dir_all(&dir);
        dir
    }

    pub(super) fn daily(cron: &str) -> Schedule {
        Schedule { kind: "cron".into(), cron: Some(cron.into()), fire_at: None, timezone: "local".into() }
    }

    pub(super) fn a_task(title: &str, cron: &str) -> ScheduledTask {
        ScheduledTask::new(title.into(), "do the thing".into(), daily(cron), String::new(), None, None)
    }

    #[test]
    pub(super) fn only_so_many_automations_may_be_switched_on() {
        let d = scratch();
        let mut store = AutomationStore::load(&d);
        for i in 0..MAX_ENABLED {
            store.create_checked(a_task(&format!("job {i}"), "0 9 * * *")).expect("under the cap");
        }
        let refused = store.create_checked(a_task("one too many", "0 9 * * *")).unwrap_err();
        assert!(refused.contains("already 20") && refused.contains("Switch one off"));
        // Switching one off makes room; switching it back on when there is none does not.
        let id = store.list()[0].id.clone();
        store.set_enabled(&id, false).unwrap();
        let extra = store.create_checked(a_task("now there's room", "0 9 * * *")).expect("room now");
        assert!(store.set_enabled(&id, true).is_err(), "the room was taken");
        assert!(store.delete(&extra.id));
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    pub(super) fn a_schedule_that_fires_too_often_never_gets_created() {
        let d = scratch();
        let mut store = AutomationStore::load(&d);
        assert!(store.create_checked(a_task("every minute", "* * * * *")).unwrap_err().contains("more often"));
        assert!(store.list().is_empty(), "nothing is written when it is refused");
        let _ = fs::remove_dir_all(&d);
    }

    pub(super) fn in_zone(cron: &str, zone: &str) -> Schedule {
        Schedule { kind: "cron".into(), cron: Some(cron.into()), fire_at: None, timezone: zone.into() }
    }

    /// Instants below were worked out with Python's `zoneinfo`, not with
    /// the code under test.
    #[test]
    pub(super) fn a_named_zone_keeps_nine_oclock_at_nine_across_daylight_saving() {
        assert_eq!(zone_of("Europe/Madrid"), Some(Zone::Named("Europe/Madrid".into())));
        assert_eq!(zone_of("Mars/Olympus_Mons"), None, "a name the zone database doesn't have is refused");
        assert_eq!(zone_of("EST"), None, "a bare abbreviation isn't a place");
        assert_eq!(zone_of("../../etc/passwd"), None);
        assert_eq!(schedule_problem(&in_zone("0 9 * * *", "Europe/Madrid")), None);
        // Said on the label, because "9am" somewhere else is a different time.
        assert_eq!(in_zone("0 9 * * *", "America/New_York").human(), format!("{} (America/New_York)", in_zone("0 9 * * *", "").human()));

        let ny = in_zone("0 9 * * *", "America/New_York");
        // Saturday 10:00 EST → Sunday 09:00, which is EDT: the clocks went forward at 02:00.
        assert_eq!(next_run_after(&ny, 1_772_895_600.0), Some(1_772_974_800.0));
        // …and the morning before is 09:00 EST, an hour later in UTC.
        assert_eq!(next_run_after(&ny, 1_772_892_000.0 - 1.0), Some(1_772_892_000.0));
    }

    #[test]
    pub(super) fn a_time_the_clock_skips_or_repeats_fires_once() {
        // 02:30 doesn't exist in New York on 8 March 2026. It fires once the clock has jumped:
        // 03:30 EDT.
        let skipped = in_zone("30 2 * * *", "America/New_York");
        assert_eq!(next_run_after(&skipped, 1_772_946_000.0), Some(1_772_955_000.0));
        // 01:30 happens twice on 1 November 2026. It fires the first time (EDT), not again an
        // hour later, and next on the 2nd.
        let repeated = in_zone("30 1 * * *", "America/New_York");
        assert_eq!(next_run_after(&repeated, 1_793_505_600.0), Some(1_793_511_000.0));
        assert_eq!(next_run_after(&repeated, 1_793_511_000.0), Some(1_793_601_000.0));
    }

    /// The machine's own clock is read through the same zone database as a
    /// named zone, so it gets the same daylight-saving rule — tested on a named zone because the
    /// machine's own can't be chosen by a test, and shown to be the same path below.
    #[test]
    pub(super) fn the_machines_clock_follows_the_same_daylight_saving_rule() {
        let ny = jiff::tz::TimeZone::get("America/New_York").unwrap();
        let at = |s: &str| chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M").unwrap();
        // Skipped on 8 March 2026: 02:30 happens at 03:30 EDT. Repeated on 1 November: the first 01:30.
        assert_eq!(instant_in(&ny, &at("2026-03-08T02:30")), Some(1_772_955_000.0));
        assert_eq!(instant_in(&ny, &at("2026-11-01T01:30")), Some(1_793_511_000.0));
        // A one-time schedule at either reads as that moment instead of being refused as unreadable.
        assert_eq!(parse_iso_in("2026-03-08T02:30", Some(&ny)), Some(1_772_955_000.0));
        assert_eq!(parse_iso_in("2026-11-01 01:30", Some(&ny)), Some(1_793_511_000.0));
        assert_eq!(parse_iso_in("2026-11-01T01:30:00-05:00", Some(&ny)), Some(1_793_514_600.0), "an offset names its own moment");

        // …and this machine's clock goes through that path: a schedule with no zone fires when
        // the same schedule named for this machine's zone does.
        // Required on macOS, where the zone is always readable, so this half can't pass by
        // quietly not running; elsewhere a machine without a named zone has nothing to compare.
        let named_system = calendar_zone(&Zone::Local).and_then(|z| z.iana_name().map(str::to_string));
        if cfg!(target_os = "macos") {
            assert!(named_system.is_some(), "this Mac's own zone should be readable from its zone database");
        }
        let Some(name) = named_system else { return };
        for cron in ["30 2 * * *", "30 1 * * *", "0 9 * * 1-5", "*/30 * * * *"] {
            let mut t = 1_772_800_000.0; // early March 2026, before the US change
            for _ in 0..12 {
                let local = next_run_after(&in_zone(cron, ""), t);
                assert_eq!(local, next_run_after(&in_zone(cron, &name), t), "{cron} after {t}");
                t = local.unwrap() + 7.0 * 86_400.0 + 1.0;
            }
        }
    }

    pub(super) fn once_in(fire_at: &str, zone: &str) -> Schedule {
        Schedule { kind: "once".into(), cron: None, fire_at: Some(fire_at.into()), timezone: zone.into() }
    }

    /// A new one-time schedule fires at the time in the zone it names; one
    /// already saved without an offset is read as it always was.
    #[test]
    pub(super) fn a_new_one_time_schedule_fires_at_that_time_in_its_own_zone() {
        // 09:00 in Madrid on 1 March 2026 is 08:00 UTC (CET, +01:00); 1 July is CEST, +02:00.
        assert_eq!(pin_fire_at("2026-03-01T09:00", "Europe/Madrid"), "2026-03-01T09:00:00+01:00");
        assert_eq!(pin_fire_at("2026-07-01 09:00", "Europe/Madrid"), "2026-07-01T09:00:00+02:00");
        assert_eq!(parse_iso_local(&pin_fire_at("2026-03-01T09:00", "Europe/Madrid")), Some(1_772_352_000.0));
        assert_eq!(pin_fire_at("2026-03-01T09:00", "+05:30"), "2026-03-01T09:00:00+05:30");
        assert_eq!(pin_fire_at("2026-03-01T09:00", "UTC"), "2026-03-01T09:00:00+00:00");
        // A time the clock skips is stored as the moment it will actually fire: 03:30 EDT.
        assert_eq!(pin_fire_at("2026-03-08T02:30", "America/New_York"), "2026-03-08T03:30:00-04:00");
        // Nothing to pin: the machine's own clock, a time that says its offset, or an unreadable one.
        assert_eq!(pin_fire_at("2026-03-01T09:00", "local"), "2026-03-01T09:00");
        assert_eq!(pin_fire_at("2026-03-01T09:00", ""), "2026-03-01T09:00");
        assert_eq!(pin_fire_at("2026-03-01T09:00:00-05:00", "Europe/Madrid"), "2026-03-01T09:00:00-05:00");
        assert_eq!(pin_fire_at("tomorrow at nine", "Europe/Madrid"), "tomorrow at nine");
        // An already-saved schedule isn't touched on read: its naive time stays on this clock.
        let saved = once_in("2026-03-01T09:00", "Europe/Madrid");
        assert_eq!(next_run_after(&saved, 0.0), parse_iso_local("2026-03-01T09:00"));
        // The label reads as the clock time and the zone it was written for.
        assert_eq!(once_in(&pin_fire_at("2026-03-01T09:00", "Europe/Madrid"), "Europe/Madrid").human(), "Once at 2026-03-01 09:00 (Europe/Madrid)");
        assert_eq!(once_in("2026-03-01T09:00:00+05:30", "local").human(), "Once at 2026-03-01 09:00 (+05:30)");
        assert_eq!(once_in("2026-03-01T09:00", "local").human(), "Once at 2026-03-01T09:00");
    }

    /// "Run now" sends the instruction through the chat socket, so the
    /// turn is where it gets its frame — and only the run's opening message does.
    #[test]
    pub(super) fn run_now_frames_the_instruction_and_nothing_typed_after_it() {
        let d = scratch();
        let mut store = AutomationStore::load(&d);
        let task = store.create_checked(a_task("morning brief", "0 9 * * *")).unwrap();
        let run = store.start_run(&task.id, "manual").unwrap();
        let sid = run.session_id.clone();

        let opening = frame_for_turn(&d, &sid, "do the thing".into());
        assert!(opening.starts_with(MANUAL_FRAME), "the instruction opens the run: {opening}");
        assert_eq!(without_frame(&opening), "do the thing");
        assert_eq!(frame_for_turn(&d, &sid, "and also check the calendar".into()), "and also check the calendar", "a follow-up is a conversation");
        assert_eq!(frame_for_turn(&d, "ordinary-chat", "do the thing".into()), "do the thing", "not a run's session");

        // Once the run has finished, the same words are a new message, not the firing.
        let mut store = AutomationStore::load(&d);
        store.finish_run(&task.id, &run.run_id, "ok", None, vec![], None).unwrap();
        assert_eq!(frame_for_turn(&d, &sid, "do the thing".into()), "do the thing");
        // And a scheduled run's session is framed by the scheduler, never twice here.
        let scheduled = store.start_run(&task.id, "schedule").unwrap();
        assert_eq!(frame_for_turn(&d, &scheduled.session_id, "do the thing".into()), "do the thing");
        let _ = fs::remove_dir_all(&d);
    }
