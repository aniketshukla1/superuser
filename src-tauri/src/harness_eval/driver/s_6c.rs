//! §6c: the informative band

use super::*;

    #[test]
    pub(super) fn history_is_a_pass_rate_per_task_over_every_model_and_run() {
        let o = vec![
            oc("a", 0, Verdict::Pass), oc("a", 1, Verdict::Fail),
            oc("b", 0, Verdict::Pass), oc("b", 1, Verdict::Pass),
        ];
        let h = History::from_outcomes("MK1", &o);
        assert_eq!(h.rates["a"], 0.5);
        assert_eq!(h.rates["b"], 1.0);
        assert_eq!(h.marker, "MK1");
        assert!(!h.measured_at.is_empty(), "a rate with no date is the stale number again");
    }

    #[test]
    pub(super) fn the_band_keeps_what_separates_models_and_drops_what_does_not() {
        let mut h = History::default();
        h.rates.insert("free".into(), 1.0);       // passes everywhere
        h.rates.insert("hopeless".into(), 0.0);   // fails everywhere
        h.rates.insert("useful".into(), 0.5);
        assert!(!h.informative("free"));
        assert!(!h.informative("hopeless"));
        assert!(h.informative("useful"));
    }

    #[test]
    pub(super) fn a_task_with_no_history_is_kept_not_dropped() {
        // Dropping it would make a newly added task invisible on the first run that
        // could measure it — the band would hide exactly what it was added to see.
        let h = History::default();
        assert!(h.informative("answers_from_truncated_output"));
    }

    #[test]
    pub(super) fn the_band_edges_are_inclusive() {
        let mut h = History::default();
        h.rates.insert("lo".into(), 0.30);
        h.rates.insert("hi".into(), 0.70);
        assert!(h.informative("lo"));
        assert!(h.informative("hi"));
    }

    #[test]
    pub(super) fn history_records_the_median_seconds_not_the_mean() {
        // One slow draw should not redefine a task's baseline. gemma4 has been
        // measured spending 108 seconds on a single step of a six-step task.
        let mut o = vec![oc("t", 0, Verdict::Pass), oc("t", 1, Verdict::Pass), oc("t", 2, Verdict::Pass)];
        o[0].seconds = 2.0;
        o[1].seconds = 3.0;
        o[2].seconds = 300.0;
        let h = History::from_outcomes("MK", &o);
        assert_eq!(h.seconds["t"], 3.0, "median, not the 101.7 a mean would give");
    }

    #[test]
    pub(super) fn a_task_that_got_much_slower_is_reported() {
        let mut prev = History::default();
        prev.seconds.insert("slow".into(), 10.0);
        prev.seconds.insert("steady".into(), 10.0);
        let mut now = History::default();
        now.seconds.insert("slow".into(), 30.0);
        now.seconds.insert("steady".into(), 11.0);
        let s = now.slower_than(&prev);
        assert_eq!(s.len(), 1, "only the 3x one: {s:?}");
        assert_eq!(s[0].0, "slow");
    }

    #[test]
    pub(super) fn a_sub_second_task_is_not_a_baseline() {
        // 0.2s to 0.4s is a doubling and means nothing.
        let mut prev = History::default();
        prev.seconds.insert("quick".into(), 0.2);
        let mut now = History::default();
        now.seconds.insert("quick".into(), 0.4);
        assert!(now.slower_than(&prev).is_empty());
    }

    #[test]
    pub(super) fn a_task_with_no_previous_time_cannot_have_regressed() {
        let prev = History::default();
        let mut now = History::default();
        now.seconds.insert("new".into(), 90.0);
        assert!(now.slower_than(&prev).is_empty());
    }
