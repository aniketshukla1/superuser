use super::*;

    fn scratch() -> std::path::PathBuf {
        static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let seq = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let d = std::env::temp_dir().join(format!("su-cal-{}-{}-{}", std::process::id(), t, seq));
        std::fs::create_dir_all(d.join("speech")).unwrap();
        d
    }

    #[test]
    fn a_measurement_survives_so_it_is_made_once_not_every_session() {
        let d = scratch();
        let c = Calibration::from_floor(0.065, 1000.0);
        save_calibration(&d, &c).unwrap();
        assert_eq!(load_calibration(&d), Some(c));
    }

    #[test]
    fn no_measurement_yet_reads_as_absent_rather_than_as_zero() {
        // Zero would look like a perfectly silent room and produce the tightest
        // possible thresholds — the opposite of a safe default.
        assert!(load_calibration(&scratch()).is_none());
    }

    #[test]
    fn a_hand_edited_calibration_that_cannot_work_is_ignored() {
        // start below end inverts the hysteresis: the turn would end the instant it
        // began. Falling back to measuring beats honouring that.
        let d = scratch();
        let bad = Calibration { noise_floor: 0.05, start_level: 0.02, end_level: 0.40, measured_at: 1.0 };
        save_calibration(&d, &bad).unwrap();
        assert!(load_calibration(&d).is_none());
    }

    #[test]
    fn a_corrupt_calibration_file_does_not_break_voice() {
        let d = scratch();
        std::fs::write(d.join("speech").join("calibration.json"), b"{ not json").unwrap();
        assert!(load_calibration(&d).is_none());
    }

    #[test]
    fn the_saved_thresholds_are_the_ones_actually_used() {
        let c = Calibration::from_floor(0.065, 0.0);
        let cfg = c.to_config();
        assert_eq!((cfg.start_level, cfg.end_level), (c.start_level, c.end_level));
        assert!(cfg.end_level > 0.065, "the measured room must be able to fall silent");
    }

    #[test]
    fn forgetting_a_calibration_returns_to_having_none() {
        let d = scratch();
        save_calibration(&d, &Calibration::from_floor(0.1, 0.0)).unwrap();
        forget_calibration(&d);
        assert!(load_calibration(&d).is_none());
    }

    #[test]
    fn the_floor_is_the_middle_of_the_sample_not_its_loudest_moment() {
        // A word spoken during calibration must not set the floor.
        let mostly_quiet = vec![0.03, 0.03, 0.04, 0.03, 0.9, 0.85, 0.03, 0.03, 0.04];
        let f = floor_of(mostly_quiet);
        assert!(f < 0.1, "speech during calibration moved the floor to {f}");
    }

    #[test]
    fn an_empty_sample_is_a_zero_floor_not_a_panic() {
        assert_eq!(floor_of(Vec::new()), 0.0);
    }


    #[test]
    fn a_turn_can_always_end_in_the_room_it_was_measured_in() {
        // The bug this fixes: the shipped end_level sat BELOW a real room's noise
        // floor, so the level never fell under it, the turn never ended, and
        // nothing was ever transcribed. Whatever the room, ending must be reachable.
        for floor in [0.0, 0.02, 0.065, 0.15, 0.3] {
            let c = TurnConfig::for_noise_floor(floor);
            assert!(c.end_level > floor, "a turn cannot end in a room at {floor}: end_level {}", c.end_level);
        }
    }

    #[test]
    fn starting_stays_out_of_reach_of_the_room() {
        for floor in [0.0, 0.02, 0.065, 0.15, 0.3] {
            let c = TurnConfig::for_noise_floor(floor);
            assert!(c.start_level > c.end_level, "the hysteresis inverted at {floor}");
            assert!(c.start_level > floor * 1.5, "room noise would start turns at {floor}");
        }
    }

    #[test]
    fn the_measured_room_reproduces_the_reported_failure_and_the_fix() {
        // Measured on the machine that reported this: avg level ~0.065.
        let floor = 0.065;
        let shipped = TurnConfig::default();
        assert!(shipped.end_level < floor, "precondition: the old default was below this room's floor");
        let fitted = TurnConfig::for_noise_floor(floor);
        assert!(fitted.end_level > floor);
    }

    #[test]
    fn a_silent_room_does_not_collapse_the_thresholds_onto_nothing() {
        // Otherwise the microphone's own hiss becomes speech.
        let c = TurnConfig::for_noise_floor(0.0);
        assert!(c.end_level >= 0.02 && c.start_level >= 0.05);
    }

    #[test]
    fn an_absurd_reading_cannot_make_the_detector_deaf() {
        // A calibration taken while someone was shouting must not leave thresholds
        // nothing can ever reach.
        let c = TurnConfig::for_noise_floor(50.0);
        assert!(c.start_level <= 1.0, "start_level {} is unreachable", c.start_level);
    }

    #[test]
    fn the_timing_is_not_disturbed_by_calibration() {
        let c = TurnConfig::for_noise_floor(0.08);
        let d = TurnConfig::default();
        assert_eq!((c.min_speech_ms, c.tail_ms), (d.min_speech_ms, d.tail_ms));
    }
