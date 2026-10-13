use super::*;

    pub(super) fn msg(role: &str, content: &str) -> Value {
        json!({ "role": role, "content": content })
    }

    #[test]
    pub(super) fn middle_out_keeps_both_ends_and_is_shorter() {
        let s = "a".repeat(1000) + &"z".repeat(1000);
        let out = middle_out(&s, 100);
        assert!(out.starts_with("aaaa"));
        assert!(out.ends_with("zzzz"));
        assert!(out.chars().count() < s.chars().count());
        assert!(out.contains("truncated"));
    }

    #[test]
    pub(super) fn middle_out_leaves_short_strings_alone() {
        assert_eq!(middle_out("hello", 100), "hello");
    }

    #[test]
    pub(super) fn middle_out_never_splits_a_multibyte_char() {
        let s = "日本語".repeat(500);
        let out = middle_out(&s, 60);
        // Round-trips as valid UTF-8 with whole characters (String guarantees it —
        // the point is that a byte-based slice here would have panicked).
        assert!(out.contains("日"));
        assert!(out.chars().count() < s.chars().count());
    }

    #[test]
    pub(super) fn truncates_openai_tool_results_but_not_the_current_step() {
        let big = "x".repeat(50_000);
        let mut msgs = vec![
            msg("user", "go"),
            json!({"role":"tool","content": big.clone()}),
            json!({"role":"tool","content": big.clone()}),
        ];
        let n = truncate_tool_results(&mut msgs, 2, TOOL_RESULT_CHAR_LIMIT, None);
        assert_eq!(n, 1, "only the historical one");
        assert!(msgs[1]["content"].as_str().unwrap().chars().count() < 50_000);
        assert_eq!(msgs[2]["content"].as_str().unwrap().chars().count(), 50_000, "current step untouched");
    }

    #[test]
    pub(super) fn truncates_claude_tool_result_blocks() {
        let big = "y".repeat(50_000);
        let mut msgs = vec![
            msg("user", "go"),
            json!({"role":"user","content":[{"type":"tool_result","tool_use_id":"t1","content": big}]}),
        ];
        let n = truncate_tool_results(&mut msgs, 2, TOOL_RESULT_CHAR_LIMIT, None);
        assert_eq!(n, 1);
        let block = &msgs[1]["content"][0];
        assert_eq!(block["type"], "tool_result", "block shape preserved");
        assert_eq!(block["tool_use_id"], "t1", "pairing id preserved");
        assert!(block["content"].as_str().unwrap().chars().count() < 50_000);
    }

    #[test]
    pub(super) fn ordinary_user_text_is_never_truncated_as_a_tool_result() {
        let long_paste = "p".repeat(50_000);
        let mut msgs = vec![msg("user", &long_paste), msg("assistant", "ok")];
        assert_eq!(truncate_tool_results(&mut msgs, 2, TOOL_RESULT_CHAR_LIMIT, None), 0);
        assert_eq!(msgs[0]["content"].as_str().unwrap().chars().count(), 50_000);
    }

    #[test]
    pub(super) fn pressure_relief_squeezes_below_budget_without_dropping_messages() {
        let big = "x".repeat(80_000);
        let mut msgs = vec![msg("user", "go")];
        for _ in 0..6 {
            msgs.push(json!({"role":"assistant","content":"","tool_calls":[{"id":"c","function":{"name":"run_shell"}}]}));
            msgs.push(json!({"role":"tool","content": big.clone()}));
        }
        // The step just executed returned something small — the usual case.
        msgs.push(json!({"role":"assistant","content":"","tool_calls":[{"id":"c","function":{"name":"read_file"}}]}));
        msgs.push(json!({"role":"tool","content":"small result"}));

        let before = msgs.len();
        assert!(estimate_messages(&msgs) > 100_000);
        relieve_pressure(&mut msgs, before - 2, 10_000, None);
        assert_eq!(msgs.len(), before, "pairing preserved — nothing removed");
        assert!(estimate_messages(&msgs) <= 10_000, "squeezed under budget");
        assert_eq!(msgs[before - 1]["content"], "small result", "current step untouched");
    }

    #[test]
    pub(super) fn pressure_relief_cannot_squeeze_the_current_step_and_says_so_by_not_lying() {
        // One enormous result from the step in flight. It is protected by design, so
        // the budget genuinely cannot be met — and the function must not start
        // deleting messages (which would orphan a tool_use) to pretend otherwise.
        let mut msgs = vec![
            msg("user", "go"),
            json!({"role":"assistant","content":"","tool_calls":[{"id":"c","function":{"name":"run_shell"}}]}),
            json!({"role":"tool","content": "x".repeat(200_000)}),
        ];
        let before = msgs.len();
        relieve_pressure(&mut msgs, before - 1, 1_000, None);
        assert_eq!(msgs.len(), before);
        assert!(estimate_messages(&msgs) > 1_000, "over budget, but structurally valid");
    }

    #[test]
    pub(super) fn last_step_start_protects_the_exchange_in_flight() {
        let msgs = vec![
            msg("user", "go"),
            json!({"role":"assistant","content":"","tool_calls":[{"id":"a"}]}),
            json!({"role":"tool","content":"old"}),
            json!({"role":"assistant","content":"","tool_calls":[{"id":"b"}]}),
            json!({"role":"tool","content":"new"}),
        ];
        assert_eq!(last_step_start(&msgs), 3, "the latest assistant turn and its result");
        // First step of a turn: nothing to protect beyond what's there.
        assert_eq!(last_step_start(&[msg("user", "go")]), 1);
        assert_eq!(last_step_start(&[]), 0);
    }

    #[test]
    pub(super) fn trim_turns_keeps_recent_whole_turns_and_returns_the_rest() {
        let mut hist = Vec::new();
        for i in 0..10 {
            hist.push(msg("user", &format!("question {i}")));
            hist.push(msg("assistant", &format!("answer {i}")));
        }
        let (kept, dropped) = trim_turns(hist, 3, 1_000_000);
        assert_eq!(kept.len(), 6, "3 turns × 2 messages");
        assert_eq!(dropped.len(), 14);
        assert_eq!(kept[0]["content"], "question 7", "kept history opens on a user turn");
        assert_eq!(dropped.last().unwrap()["content"], "answer 6", "dropped ends where kept begins");
    }

    #[test]
    pub(super) fn trim_turns_respects_the_token_budget_too() {
        let long = "w ".repeat(4_000); // ~2000 tokens per message
        let mut hist = Vec::new();
        for _ in 0..10 {
            hist.push(msg("user", &long));
            hist.push(msg("assistant", &long));
        }
        let (kept, dropped) = trim_turns(hist, 100, 9_000);
        assert!(!kept.is_empty());
        assert!(!dropped.is_empty(), "token budget bit before the turn cap");
        assert!(estimate_messages(&kept) <= 9_000);
        assert_eq!(kept[0]["role"], "user");
    }

    #[test]
    pub(super) fn trim_turns_always_keeps_the_last_turn_even_if_it_blows_the_budget() {
        let huge = "w".repeat(400_000);
        let hist = vec![msg("user", "old"), msg("assistant", "old"), msg("user", &huge)];
        let (kept, dropped) = trim_turns(hist, 5, 100);
        assert_eq!(kept.len(), 1, "the newest turn survives");
        assert_eq!(kept[0]["content"].as_str().unwrap().chars().count(), 400_000);
        assert_eq!(dropped.len(), 2);
    }

    #[test]
    pub(super) fn trim_turns_is_a_no_op_under_budget() {
        let hist = vec![msg("user", "hi"), msg("assistant", "hello")];
        let (kept, dropped) = trim_turns(hist.clone(), 20, 48_000);
        assert_eq!(kept, hist);
        assert!(dropped.is_empty());
    }

    #[test]
    pub(super) fn claude_tool_results_do_not_count_as_turn_boundaries() {
        // A Claude-shaped history: one real turn whose tool results ride on `user`.
        let hist = vec![
            msg("user", "first"),
            msg("assistant", "a1"),
            msg("user", "second"),
            json!({"role":"assistant","content":[{"type":"tool_use","id":"t1","name":"read_file"}]}),
            json!({"role":"user","content":[{"type":"tool_result","tool_use_id":"t1","content":"data"}]}),
            msg("assistant", "a2"),
        ];
        let (kept, _) = trim_turns(hist, 1, 1_000_000);
        assert_eq!(kept.len(), 4, "the tool exchange stays with its own turn");
        assert_eq!(kept[0]["content"], "second");
    }
