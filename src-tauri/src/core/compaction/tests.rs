use super::*;

    fn openai_call(name: &str, args: &str) -> Value {
        json!({"role": "assistant", "tool_calls": [{"function": {"name": name, "arguments": args}}]})
    }

    fn claude_call(name: &str, input: Value) -> Value {
        json!({"role": "assistant", "content": [{"type": "tool_use", "name": name, "input": input}]})
    }

    #[test]
    fn calls_are_read_out_of_both_provider_shapes() {
        let msgs = vec![
            openai_call("read_file", r#"{"path":"a.md"}"#),
            claude_call("grep", json!({"pattern": "needle"})),
            json!({"role": "user", "content": "unrelated"}),
        ];
        let calls = calls_in(&msgs);
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].0, "read_file");
        assert_eq!(calls[1].0, "grep");
    }

    #[test]
    fn asking_again_for_what_compaction_took_away_is_a_refetch() {
        // The paper's "regressive exploration": +0.031 refetches per step right
        // after a boundary. This is that signal, counted without a judge.
        let dropped = vec![openai_call("read_file", r#"{"path":"a.md"}"#)];
        let mut w = Watch::open(Boundary { turns_dropped: 4, chars_dropped: 9000, summarized: true }, &dropped);
        w.observe("read_file", r#"{"path":"a.md"}"#, "# contents");
        assert_eq!(w.refetches(), 1);
    }

    #[test]
    fn a_different_argument_is_not_a_refetch() {
        let dropped = vec![openai_call("read_file", r#"{"path":"a.md"}"#)];
        let mut w = Watch::open(Boundary::default(), &dropped);
        w.observe("read_file", r#"{"path":"b.md"}"#, "# other");
        assert_eq!(w.refetches(), 0, "reading a different file is ordinary work");
    }

    #[test]
    fn a_refusal_counts_however_it_is_worded() {
        let mut w = Watch::open(Boundary::default(), &[]);
        w.observe("run_shell", "{}", "error: that command needs approval");
        w.observe("write_file", "{}", "refused: outside the workspace");
        w.observe("send_message", "{}", "that target is not allowed here");
        assert_eq!(w.blocked(), 3);
    }

    #[test]
    fn the_window_is_three_steps_and_then_it_stops_counting() {
        // Past the window the agent has re-established where it is, and anything
        // counted would be ordinary work rather than recovery from the boundary.
        let dropped = vec![openai_call("read_file", r#"{"path":"a.md"}"#)];
        let mut w = Watch::open(Boundary::default(), &dropped);
        for _ in 0..WATCH_STEPS {
            w.observe("read_file", r#"{"path":"a.md"}"#, "x");
            w.step_done();
        }
        assert!(!w.open_still());
        w.observe("read_file", r#"{"path":"a.md"}"#, "x");
        assert_eq!(w.refetches(), WATCH_STEPS, "the fourth step is not counted");
    }

    #[test]
    fn a_finished_turn_can_be_scored_in_one_pass() {
        let dropped = vec![openai_call("read_file", r#"{"path":"a.md"}"#)];
        let mut w = Watch::open(Boundary { turns_dropped: 3, chars_dropped: 500, summarized: true }, &dropped);
        // The turn asks again for the file compaction took away, then gets refused
        // on a shell command, then does ordinary work.
        let turn = vec![
            openai_call("read_file", r#"{"path":"a.md"}"#),
            json!({"role": "tool", "content": "# a"}),
            openai_call("run_shell", r#"{"command":"rm -rf /"}"#),
            json!({"role": "tool", "content": "error: that command needs approval"}),
            openai_call("grep", r#"{"pattern":"x"}"#),
            json!({"role": "tool", "content": "3 matches"}),
        ];
        w.observe_turn(&turn);
        assert_eq!(w.refetches(), 1, "the re-read of the dropped file");
        assert_eq!(w.blocked(), 1, "the refused shell call");
        assert_eq!(w.frame()["watched_steps"], json!(3));
    }

    #[test]
    fn scoring_a_turn_stops_at_the_window_even_with_more_steps_in_it() {
        let dropped = vec![openai_call("read_file", r#"{"path":"a.md"}"#)];
        let mut w = Watch::open(Boundary::default(), &dropped);
        let mut turn = Vec::new();
        for _ in 0..6 {
            turn.push(openai_call("read_file", r#"{"path":"a.md"}"#));
            turn.push(json!({"role": "tool", "content": "# a"}));
        }
        w.observe_turn(&turn);
        assert_eq!(w.refetches(), WATCH_STEPS, "six steps, three counted");
    }

    #[test]
    fn the_frame_says_whether_anything_was_summarized_at_all() {
        // FIFO and summary are different arms with very different measured
        // outcomes, so a frame that could not tell them apart would be useless for
        // the comparison this exists to enable.
        let fifo = Watch::open(Boundary { turns_dropped: 2, chars_dropped: 10, summarized: false }, &[]);
        assert_eq!(fifo.frame()["summarized"], json!(false));
        let summed = Watch::open(Boundary { turns_dropped: 2, chars_dropped: 10, summarized: true }, &[]);
        assert_eq!(summed.frame()["summarized"], json!(true));
    }

    #[test]
    fn a_boundary_that_dropped_no_calls_still_records_the_size_of_what_went() {
        let w = Watch::open(Boundary { turns_dropped: 6, chars_dropped: 12_000, summarized: true }, &[]);
        let f = w.frame();
        assert_eq!(f["turns_dropped"], json!(6));
        assert_eq!(f["chars_dropped"], json!(12_000));
        assert_eq!(f["calls_lost"], json!(0));
    }
