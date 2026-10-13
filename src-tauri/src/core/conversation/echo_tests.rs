use super::*;

    #[test]
    fn a_gap_between_sentences_still_counts_as_the_assistant_speaking() {
        // The player for one sentence exits before the next starts, so a bare
        // "is it speaking" check says no in the middle of an answer — and the
        // detector would go back to trusting the microphone mid-reply.
        let mut g = EchoGate::default();
        assert!(g.tick(true, 20));
        for _ in 0..10 {
            assert!(g.tick(false, 20), "a 200ms gap between sentences read as silence");
        }
    }

    #[test]
    fn the_room_stops_ringing_eventually() {
        let mut g = EchoGate::default();
        g.tick(true, 20);
        let mut ticks = 0;
        while g.tick(false, 20) {
            ticks += 1;
            assert!(ticks < 200, "the gate never released");
        }
        // Released after roughly the hangover, not immediately and not forever.
        assert!((ticks as u32 * 20) >= ECHO_HANGOVER_MS - 20);
        assert!(!g.active());
    }

    #[test]
    fn silence_before_the_assistant_ever_spoke_is_not_gated() {
        let mut g = EchoGate::default();
        for _ in 0..50 {
            assert!(!g.tick(false, 20), "gated audio the assistant never made");
        }
    }

    #[test]
    fn speaking_again_restarts_the_hangover() {
        let mut g = EchoGate::with_hangover(100);
        g.tick(true, 20);
        g.tick(false, 60); // partway through the hangover
        g.tick(true, 20); // it speaks again
        assert!(g.tick(false, 60), "the countdown should have restarted");
    }

    #[test]
    fn a_turn_never_starts_inside_the_assistants_own_voice() {
        // The failure this prevents: the mic records the assistant, the pre-roll
        // rewinds into that recording, whisper transcribes it, and it arrives as
        // if the user had said it.
        let barrier = 1000;
        assert_eq!(turn_start_index(1100, 500, 0, barrier), barrier);
    }

    #[test]
    fn the_pre_roll_still_applies_when_nothing_is_in_the_way() {
        assert_eq!(turn_start_index(5000, 800, 0, 0), 4200);
    }

    #[test]
    fn a_finished_turn_is_never_transcribed_twice() {
        assert_eq!(turn_start_index(5000, 4000, 3000, 0), 3000);
    }

    #[test]
    fn the_start_can_never_run_past_the_end_of_the_buffer() {
        // A barrier set while the buffer was longer must not produce a backwards
        // slice after the buffer is trimmed.
        assert_eq!(turn_start_index(100, 10, 0, 9_999), 100);
    }

    #[test]
    fn the_highest_floor_wins() {
        // consumed beats pre-roll, barrier beats consumed.
        assert_eq!(turn_start_index(9000, 1000, 8500, 0), 8500);
        assert_eq!(turn_start_index(9000, 1000, 8500, 8800), 8800);
    }
