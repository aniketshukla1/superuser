use super::*;

    #[test]
    fn ansi_colour_is_stripped_but_the_words_survive() {
        let raw = "\x1b[32mpassed\x1b[0m 12 tests";
        assert_eq!(strip_ansi(raw), "passed 12 tests");
    }

    #[test]
    fn an_osc_title_sequence_does_not_leak_into_the_output() {
        let raw = "\x1b]0;some window title\x07hello";
        assert_eq!(strip_ansi(raw), "hello");
    }

    #[test]
    fn the_echoed_command_and_the_sentinel_are_both_dropped() {
        let nonce = "abc";
        // What the pty really returns: the echoed line (sentinel still literal `%d`),
        // the command's own output, then the expanded marker.
        let raw = format!(
            "$ ls\nprintf '\\n__SU_END_{nonce}_%d__\\n' $?\nfile-one.txt\nfile-two.txt\n__SU_END_{nonce}_0__\n"
        );
        assert_eq!(clean(&raw, nonce), "file-one.txt\nfile-two.txt");
    }

    #[test]
    fn a_failing_command_still_yields_its_stderr() {
        let nonce = "d00d";
        let raw = format!("$ cat nope\ncat: nope: No such file or directory\n__SU_END_{nonce}_1__\n");
        assert_eq!(clean(&raw, nonce), "cat: nope: No such file or directory");
    }

    #[test]
    fn feeding_the_echo_alone_does_not_complete_the_command() {
        // Guards the `%d`-vs-digits distinction: the echo must not be mistaken for
        // the real marker, or every command would "finish" the instant it was typed.
        *lock(&shared().capture) =
            Some(Capture { nonce: "z9".into(), buf: String::new(), code: None });
        feed("printf '\\n__SU_END_z9_%d__\\n' $?\n");
        assert!(lock(&shared().capture).as_ref().unwrap().code.is_none());
        feed("output here\n__SU_END_z9_7__\n");
        assert_eq!(lock(&shared().capture).as_ref().unwrap().code, Some(7));
        *lock(&shared().capture) = None;
    }

    #[test]
    fn with_no_terminal_open_the_agent_is_told_to_use_its_own_process() {
        close();
        assert!(run_captured("echo hi", 5).is_none());
    }
