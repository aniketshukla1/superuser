use super::*;

    fn cwd() -> String {
        std::env::temp_dir().to_string_lossy().to_string()
    }

    /// Poll until `cond` holds on the accumulated output, or give up.
    fn until(session: &str, h: &str, cond: impl Fn(&str, Option<i32>) -> bool) -> (String, Option<i32>) {
        let mut all = String::new();
        let mut exit = None;
        for _ in 0..100 {
            let (chunk, e) = read(session, h).unwrap();
            all.push_str(&chunk);
            exit = e;
            if cond(&all, exit) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        (all, exit)
    }

    #[test]
    fn a_process_keeps_running_and_its_output_arrives_in_pieces() {
        let h = start("s-run", &cwd(), "echo first; sleep 0.3; echo second").unwrap();
        let (all, exit) = until("s-run", &h, |t, e| t.contains("second") && e.is_some());
        assert!(all.contains("first") && all.contains("second"), "{all:?}");
        assert_eq!(exit, Some(0));
        // A second read returns only what is new — which, after exit, is nothing.
        let (again, _) = read("s-run", &h).unwrap();
        assert!(!again.contains("first"), "a read must not repeat what was already returned: {again:?}");
        let _ = stop("s-run", &h);
    }

    #[test]
    fn input_reaches_an_interactive_process() {
        // The capability run_shell cannot provide: a process waiting on its input.
        let h = start("s-repl", &cwd(), "cat").unwrap();
        send("s-repl", &h, "ping-from-the-agent", true).unwrap();
        let (all, _) = until("s-repl", &h, |t, _| t.contains("ping-from-the-agent"));
        assert!(all.contains("ping-from-the-agent"), "{all:?}");
        stop("s-repl", &h).unwrap();
        assert!(read("s-repl", &h).is_err(), "a stopped process is gone");
    }

    #[test]
    fn the_gate_holds_on_the_command_and_on_everything_sent() {
        assert!(start("s-gate", &cwd(), "rm -rf /").unwrap_err().starts_with("blocked"));
        // Starting a shell and sending it the refused command must not be a way round.
        let h = start("s-gate", &cwd(), "cat").unwrap();
        assert!(send("s-gate", &h, "rm -rf /", true).unwrap_err().starts_with("blocked"));
        let _ = stop("s-gate", &h);
    }

    #[test]
    fn one_conversation_cannot_see_anothers_processes() {
        let h = start("s-mine", &cwd(), "cat").unwrap();
        let err = read("s-theirs", &h).unwrap_err();
        assert!(err.contains("no process"), "must read as unknown, not forbidden: {err}");
        assert!(send("s-theirs", &h, "x", true).is_err());
        assert!(stop("s-theirs", &h).is_err());
        assert!(list("s-theirs").is_empty());
        let _ = stop("s-mine", &h);
    }

    #[test]
    fn there_is_a_ceiling_on_live_processes() {
        let hs: Vec<String> = (0..MAX_PER_SESSION).map(|_| start("s-cap", &cwd(), "cat").unwrap()).collect();
        let err = start("s-cap", &cwd(), "cat").unwrap_err();
        assert!(err.contains("already running"), "{err}");
        for h in hs {
            let _ = stop("s-cap", &h);
        }
    }
