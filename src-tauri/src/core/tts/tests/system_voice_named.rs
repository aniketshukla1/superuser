use super::*;

    #[test]
    pub(super) fn an_unknown_engine_falls_back_to_the_one_that_needs_no_setup() {
        assert_eq!(Engine::parse("piper"), Engine::Piper);
        assert_eq!(Engine::parse("KOKORO"), Engine::Kokoro);
        assert_eq!(Engine::parse("elevenlabs"), Engine::Cloud);
        assert_eq!(Engine::parse("nonsense"), Engine::System);
        assert_eq!(Engine::parse(""), Engine::System);
    }

    #[test]
    pub(super) fn only_the_cloud_engine_sends_text_off_the_machine() {
        assert!(Engine::Cloud.leaves_machine());
        for e in [Engine::System, Engine::Piper, Engine::Kokoro] {
            assert!(!e.leaves_machine(), "{e:?} should be local");
        }
    }

    #[test]
    pub(super) fn the_first_sentence_is_speakable_before_the_rest_arrives() {
        // The whole point: speech starts on sentence one, mid-stream.
        let (chunk, used) = next_chunk("Hello there. And then", false).unwrap();
        assert_eq!(chunk, "Hello there.");
        assert_eq!(&"Hello there. And then"[used..], " And then");
    }

    #[test]
    pub(super) fn a_half_written_sentence_waits_rather_than_being_spoken_in_pieces() {
        assert!(next_chunk("I was just about to", false).is_none());
    }

    #[test]
    pub(super) fn the_end_of_the_stream_speaks_what_is_left_even_without_a_full_stop() {
        let (chunk, _) = next_chunk("no final period here", true).unwrap();
        assert_eq!(chunk, "no final period here");
        // ...but an empty tail is nothing to say.
        assert!(next_chunk("   ", true).is_none());
    }

    #[test]
    pub(super) fn a_version_number_does_not_break_a_sentence_in_half() {
        assert!(next_chunk("Upgrading to v1.2 of the", false).is_none());
    }

    #[test]
    pub(super) fn an_abbreviation_does_not_end_a_sentence() {
        assert!(next_chunk("Ask Dr. Smith about", false).is_none());
        assert!(next_chunk("Use e.g. the second one", false).is_none());
    }

    #[test]
    pub(super) fn question_and_exclamation_end_a_sentence_too() {
        assert_eq!(next_chunk("Ready? Then go", false).unwrap().0, "Ready?");
        assert_eq!(next_chunk("Done! Next up", false).unwrap().0, "Done!");
    }

    #[test]
    pub(super) fn a_closing_quote_stays_with_its_sentence() {
        let (chunk, _) = next_chunk("She said \"go.\" Then left", false).unwrap();
        assert_eq!(chunk, "She said \"go.\"");
    }

    #[test]
    pub(super) fn code_blocks_and_tables_are_not_read_aloud() {
        let md = "Here is the fix.\n\n```rust\nfn main() {}\n```\n\n| a | b |\n\nThat's all.";
        let out = speakable(md);
        assert!(out.contains("Here is the fix."));
        assert!(out.contains("That's all."));
        assert!(out.contains("code omitted"));
        assert!(!out.contains("fn main"), "code was read aloud: {out}");
        assert!(!out.contains('|'), "a table row was read aloud: {out}");
    }

    #[test]
    pub(super) fn emphasis_marks_are_not_pronounced() {
        assert_eq!(speakable("This is **really** good"), "This is really good");
        assert_eq!(speakable("## Heading"), "Heading");
    }

    #[test]
    pub(super) fn the_system_voice_is_named_per_platform_not_assumed_to_be_macos() {
        let (program, args) = system_command("Daniel", "hello");
        if cfg!(target_os = "macos") {
            assert_eq!(program, "say");
            assert_eq!(args, vec!["-v", "Daniel", "hello"]);
        } else if cfg!(target_os = "windows") {
            assert_eq!(program, "powershell");
            assert!(args.last().unwrap().contains("SpeechSynthesizer"));
        } else {
            assert_eq!(program, "spd-say");
        }
    }

    #[test]
    pub(super) fn an_apostrophe_cannot_escape_the_windows_command() {
        // Only meaningful on the PowerShell path, but the escaping is worth pinning
        // wherever the test runs: the text is data, never script.
        let (_, args) = system_command("", "it's fine'; Remove-Item C:\\ #");
        let joined = args.join(" ");
        if cfg!(target_os = "windows") {
            assert!(joined.contains("it''s fine''"), "quotes were not doubled: {joined}");
        } else {
            assert!(joined.contains("it's fine"));
        }
    }
