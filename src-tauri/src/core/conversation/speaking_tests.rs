use super::*;

    #[test]
    fn interrupting_takes_more_than_the_assistants_own_voice() {
        // The failure this prevents: the speakers feed the mic, the detector fires
        // on the assistant's first sentence, and it silences itself mid-word.
        let normal = TurnConfig::default();
        let deafened = normal.while_speaking();
        assert!(deafened.start_level > normal.start_level, "barge-in must need more than ordinary speech");
        assert!(deafened.min_speech_ms > normal.min_speech_ms, "and must last longer than a cadence gap");

        // Its own voice, at a level that would start a turn in silence, must not.
        let mut d = TurnDetector::new(deafened);
        let own_voice = normal.start_level * 1.5;
        for _ in 0..30 {
            assert_eq!(d.push(own_voice, 20), TurnEvent::Nothing, "the assistant interrupted itself");
        }
    }

    #[test]
    fn a_person_talking_over_it_still_gets_through() {
        // Deafening it must not make it deaf: real speech close to the mic is well
        // above the threshold and does interrupt.
        let mut d = TurnDetector::new(TurnConfig::default().while_speaking());
        let events: Vec<TurnEvent> = (0..20).map(|_| d.push(0.9, 20)).filter(|e| *e != TurnEvent::Nothing).collect();
        assert_eq!(events, vec![TurnEvent::SpeechStarted]);
    }

    #[test]
    fn the_recording_rewinds_so_the_first_syllable_is_not_clipped() {
        // 300ms at 16kHz is 4800 samples of lead-in.
        assert_eq!(pre_roll_samples(16_000), 4800);
        assert_eq!(pre_roll_samples(48_000), 14_400);
        // And the rewind is longer than the delay before the detector can fire,
        // otherwise it would still start mid-word.
        assert!(PRE_ROLL_MS > TurnConfig::default().min_speech_ms, "pre-roll must cover the detection delay");
    }
