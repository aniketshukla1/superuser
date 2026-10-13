use super::*;

    fn scratch() -> PathBuf {
        static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let seq = N.fetch_add(1, Ordering::SeqCst);
        let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let d = std::env::temp_dir().join(format!("su-dict-{}-{}-{}", std::process::id(), t, seq));
        let _ = std::fs::create_dir_all(d.join("speech"));
        d
    }

    #[test]
    fn stereo_is_mixed_down_before_anything_else() {
        // L and R average to the middle, and two channels of four samples is two
        // frames.
        let out = to_whisper_mono(&[1.0, 0.0, 0.0, 1.0], 2, TARGET_RATE);
        assert_eq!(out, vec![0.5, 0.5]);
    }

    #[test]
    fn audio_already_at_the_right_rate_is_left_alone() {
        let input: Vec<f32> = (0..100).map(|i| i as f32).collect();
        assert_eq!(to_whisper_mono(&input, 1, TARGET_RATE), input);
    }

    #[test]
    fn forty_eight_kilohertz_becomes_sixteen() {
        let input: Vec<f32> = vec![0.5; 4800]; // 0.1s at 48kHz
        let out = to_whisper_mono(&input, 1, 48_000);
        assert_eq!(out.len(), 1600, "0.1s at 16kHz is 1600 samples");
        // A constant signal must survive resampling unchanged.
        assert!(out.iter().all(|s| (*s - 0.5).abs() < 1e-6));
    }

    #[test]
    fn downsampling_averages_rather_than_dropping_samples() {
        // 3:1. Naive picking would return the first of each triple (1.0, 1.0);
        // averaging returns their mean, which is what keeps aliasing out.
        let input = vec![1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
        let out = to_whisper_mono(&input, 1, TARGET_RATE * 3);
        assert_eq!(out.len(), 2);
        for s in &out {
            assert!((s - 1.0 / 3.0).abs() < 1e-6, "expected the window mean, got {s}");
        }
    }

    #[test]
    fn empty_or_nonsense_input_yields_nothing_rather_than_panicking() {
        assert!(to_whisper_mono(&[], 1, TARGET_RATE).is_empty());
        assert!(to_whisper_mono(&[1.0], 0, TARGET_RATE).is_empty());
        assert!(to_whisper_mono(&[1.0], 1, 0).is_empty());
    }

    #[test]
    fn silence_reads_as_no_level_and_speech_reads_as_some() {
        assert_eq!(level_of(&[0.0; 100]), 0.0);
        assert_eq!(level_of(&[]), 0.0);
        let speechish = level_of(&[0.1, -0.1, 0.12, -0.09]);
        assert!(speechish > 0.0 && speechish < 1.0, "got {speechish}");
        // A shout clamps rather than overflowing the meter.
        assert_eq!(level_of(&[1.0, -1.0, 1.0]), 1.0);
    }

    #[test]
    fn whispers_own_stage_directions_are_not_typed_into_the_composer() {
        assert_eq!(clean_transcript("[BLANK_AUDIO]"), "");
        assert_eq!(clean_transcript("Hello (door closes) there"), "Hello there");
        assert_eq!(clean_transcript("  [MUSIC]  Book the room.  "), "Book the room.");
    }

    #[test]
    fn a_truncated_download_is_reported_unverified_so_repair_is_offered() {
        let d = scratch();
        std::fs::write(model_path(&d), vec![0u8; 1024]).unwrap();
        mark_verified(&d).unwrap();
        let s = Dictation::default().status(&d, "test".into(), None);
        assert!(s.model_installed, "the file is there");
        assert!(!s.model_verified, "but it is far too small to be the model");
        assert!(!model_looks_complete(&d));
    }

    #[test]
    fn a_model_is_only_verified_once_it_has_been_checked() {
        let d = scratch();
        std::fs::write(model_path(&d), vec![0u8; MIN_PLAUSIBLE_BYTES as usize + 1]).unwrap();
        let dict = Dictation::default();
        assert!(!dict.status(&d, "t".into(), None).model_verified, "no marker yet");
        mark_verified(&d).unwrap();
        assert!(dict.status(&d, "t".into(), None).model_verified);
    }

    #[test]
    fn forgetting_the_model_clears_its_markers_too() {
        let d = scratch();
        std::fs::write(model_path(&d), vec![0u8; MIN_PLAUSIBLE_BYTES as usize + 1]).unwrap();
        mark_verified(&d).unwrap();
        mark_tested(&d).unwrap();
        forget_model(&d);
        let s = Dictation::default().status(&d, "t".into(), None);
        assert!(!s.model_installed && !s.model_verified && !s.test_passed);
    }

    #[test]
    fn an_unsupported_device_says_why_rather_than_just_failing() {
        let d = scratch();
        let s = Dictation::default().status(&d, "No input device".into(), Some("no microphone found".into()));
        assert!(!s.supported);
        assert_eq!(s.compatibility_reason.as_deref(), Some("no microphone found"));
    }

    #[test]
    fn the_level_meter_round_trips_through_its_atomic() {
        let dict = Dictation::default();
        dict.set_level(0.25);
        assert!((dict.level() - 0.25).abs() < 0.002);
        dict.set_level(5.0);
        assert_eq!(dict.level(), 1.0, "the meter clamps");
    }
