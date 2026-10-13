use super::*;

    /// Push `n` frames of one level, returning every event that fired.
    fn feed(d: &mut TurnDetector, level: f32, frames: u32, frame_ms: u32) -> Vec<TurnEvent> {
        (0..frames)
            .map(|_| d.push(level, frame_ms))
            .filter(|e| *e != TurnEvent::Nothing)
            .collect()
    }

    #[test]
    fn a_cough_does_not_start_a_turn() {
        // One loud frame, then quiet. Nothing should fire — otherwise a chair creak
        // interrupts the assistant mid-answer.
        let mut d = TurnDetector::new(TurnConfig::default());
        assert_eq!(d.push(0.9, 20), TurnEvent::Nothing);
        assert!(feed(&mut d, 0.0, 10, 20).is_empty());
        assert!(!d.speaking());
    }

    #[test]
    fn sustained_speech_starts_a_turn() {
        let mut d = TurnDetector::new(TurnConfig::default());
        let events = feed(&mut d, 0.3, 10, 20); // 200ms > min_speech_ms
        assert_eq!(events, vec![TurnEvent::SpeechStarted]);
        assert!(d.speaking());
    }

    #[test]
    fn a_turn_ends_only_after_the_silence_tail() {
        let mut d = TurnDetector::new(TurnConfig::default());
        feed(&mut d, 0.3, 10, 20);
        // 300ms of quiet is a pause for breath, not the end of a turn.
        assert!(feed(&mut d, 0.0, 15, 20).is_empty());
        assert!(d.speaking(), "cut the user off mid-thought");
        // Past the tail, the turn is over.
        let events = feed(&mut d, 0.0, 25, 20);
        assert_eq!(events, vec![TurnEvent::SpeechEnded]);
        assert!(!d.speaking());
    }

    #[test]
    fn pausing_mid_sentence_does_not_end_the_turn() {
        // The behaviour people notice most: think for half a second, keep talking.
        let mut d = TurnDetector::new(TurnConfig::default());
        feed(&mut d, 0.3, 10, 20);
        feed(&mut d, 0.0, 20, 20); // 400ms pause, under the 700ms tail
        feed(&mut d, 0.3, 10, 20); // carry on
        assert!(d.speaking());
        // ...and the tail restarts from the new silence, not from the old pause.
        assert!(feed(&mut d, 0.0, 20, 20).is_empty());
    }

    #[test]
    fn the_two_thresholds_stop_the_detector_flapping() {
        // Between end_level and start_level: too quiet to start a turn, loud enough
        // to keep one alive. A single threshold here would toggle every frame.
        let mut d = TurnDetector::new(TurnConfig::default());
        assert!(feed(&mut d, 0.07, 50, 20).is_empty(), "room noise started a turn");
        feed(&mut d, 0.3, 10, 20);
        assert!(feed(&mut d, 0.07, 60, 20).is_empty(), "room noise ended a turn");
        assert!(d.speaking());
    }

    #[test]
    fn speech_during_an_answer_is_reported_so_it_can_barge_in() {
        // Same event; the caller decides it means "stop talking".
        let mut d = TurnDetector::new(TurnConfig::default());
        assert_eq!(feed(&mut d, 0.4, 10, 20), vec![TurnEvent::SpeechStarted]);
    }

    #[test]
    fn resetting_forgets_a_half_finished_turn() {
        let mut d = TurnDetector::new(TurnConfig::default());
        feed(&mut d, 0.3, 10, 20);
        d.reset();
        assert!(!d.speaking());
        assert!(feed(&mut d, 0.0, 50, 20).is_empty(), "a reset turn should not later 'end'");
    }

    #[test]
    fn an_answer_spoken_in_full_was_not_interrupted() {
        let mut u = Unsaid::begin("One. Two. Three.");
        u.spoke("One.");
        u.spoke("Two.");
        u.spoke("Three.");
        assert!(!u.was_interrupted());
        assert_eq!(u.unheard(), "");
        assert!(u.note().is_none());
    }

    #[test]
    fn being_cut_off_keeps_what_the_user_never_heard() {
        let mut u = Unsaid::begin("First point. Second point. Third point.");
        u.spoke("First point.");
        assert!(u.was_interrupted());
        assert_eq!(u.heard(), "First point.");
        assert_eq!(u.unheard(), "Second point. Third point.");
        assert!(u.note().unwrap().contains("Second point."));
    }

    #[test]
    fn the_transcript_records_what_was_said_aloud_not_what_was_generated() {
        // The written record must not claim the user heard a paragraph that was
        // cut off after one sentence.
        let mut u = Unsaid::begin("Alpha. Beta.");
        u.spoke("Alpha.");
        assert_eq!(u.note(), Some("(interrupted — unsaid: Beta.)".to_string()));
    }

    #[test]
    fn a_chunk_reworded_on_its_way_to_the_voice_does_not_lose_the_thread() {
        // `speakable()` strips markdown, so what reaches the speaker may not match
        // the source text. The remainder should still be sensible rather than empty.
        let mut u = Unsaid::begin("**Alpha.** Beta.");
        u.spoke("Alpha.");
        assert!(u.was_interrupted());
        assert!(u.unheard().contains("Beta."));
    }

    #[test]
    fn a_persona_sounds_the_same_every_time_you_talk_to_it() {
        let voices: Vec<String> = ["Daniel", "Karen", "Moira", "Rishi"].iter().map(|s| s.to_string()).collect();
        let a = voice_for_persona("coder", &voices).unwrap();
        let b = voice_for_persona("coder", &voices).unwrap();
        assert_eq!(a, b, "a colleague whose voice changed each session would be unsettling");
        assert!(voices.contains(&a));
    }

    #[test]
    fn different_personas_generally_get_different_voices() {
        let voices: Vec<String> = ["Daniel", "Karen", "Moira", "Rishi", "Tessa", "Fiona"].iter().map(|s| s.to_string()).collect();
        let picks: std::collections::HashSet<String> = ["assistant", "coder", "ops", "finance"]
            .iter()
            .filter_map(|p| voice_for_persona(p, &voices))
            .collect();
        assert!(picks.len() >= 3, "four personas collapsed into {} voice(s)", picks.len());
    }

    #[test]
    fn no_voices_installed_is_not_a_crash() {
        assert!(voice_for_persona("coder", &[]).is_none());
    }

    #[test]
    fn persona_naming_is_not_case_sensitive() {
        let voices: Vec<String> = ["A", "B", "C"].iter().map(|s| s.to_string()).collect();
        assert_eq!(voice_for_persona("Coder", &voices), voice_for_persona("coder", &voices));
    }
