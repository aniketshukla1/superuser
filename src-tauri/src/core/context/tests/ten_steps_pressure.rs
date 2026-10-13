use super::*;

    #[test]
    pub(super) fn budget_for_a_local_model_is_capped_by_num_ctx() {
        let mut s = Settings::default();
        s.num_ctx = 16384;
        s.max_tokens = 4096;
        s.agent_max_context_tokens = 48000;
        assert_eq!(Budget::resolve(&s, true).max_tokens, 16384 - 4096);
        assert_eq!(Budget::resolve(&s, false).max_tokens, 48000, "cloud uses the configured budget");
    }

    #[test]
    pub(super) fn absurd_num_ctx_still_leaves_a_usable_budget() {
        let mut s = Settings::default();
        s.num_ctx = 1024;
        s.max_tokens = 4096;
        assert_eq!(Budget::resolve(&s, true).max_tokens, 2_000, "floored, not zero");
    }

    #[test]
    pub(super) fn stats_report_usage_against_the_budget() {
        let hist = vec![msg("user", &"a".repeat(4_000)), msg("assistant", "ok")];
        let st = stats(&hist, Budget { max_tokens: 10_000, max_turns: 20, max_steps: 24 });
        assert_eq!(st.messages, 2);
        assert_eq!(st.user, 1);
        assert_eq!(st.assistant, 1);
        assert!(st.est_tokens >= 1_000 && st.est_tokens < 1_100);
        assert_eq!(st.pct, 10);
    }

    /// A scratch data dir, same shape the rest of the tree uses.
    pub(super) fn obs_scratch() -> std::path::PathBuf {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static N: AtomicUsize = AtomicUsize::new(0);
        let d = std::env::temp_dir().join(format!(
            "su-ctx-obs-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        let _ = std::fs::create_dir_all(&d);
        d
    }

    #[test]
    pub(super) fn a_truncated_result_leaves_a_handle_that_returns_the_whole_thing() {
        // The point of §2: what was cut is still reachable, and reachable as what
        // the model actually saw rather than as whatever the source says now.
        let store = crate::core::observations::ObservationStore::for_session(&obs_scratch(), "s");
        let long: String = (0..5_000).map(|i| format!("line {i}\n")).collect();
        let out = middle_out_recorded(&long, 400, Some(&store));

        assert!(out.contains("recall_output(id="), "the notice must name a handle: {out:?}");
        let id = out.split("recall_output(id=\"").nth(1).and_then(|r| r.split('"').next()).unwrap();
        assert_eq!(store.get(id).unwrap(), long, "the handle returns the whole result");
    }

    #[test]
    pub(super) fn the_same_output_truncated_twice_yields_one_handle() {
        let store = crate::core::observations::ObservationStore::for_session(&obs_scratch(), "s");
        let long: String = (0..5_000).map(|i| format!("line {i}\n")).collect();
        let a = middle_out_recorded(&long, 400, Some(&store));
        let b = middle_out_recorded(&long, 400, Some(&store));
        assert_eq!(a, b, "content-addressed, so the notice is stable across steps");
    }

    #[test]
    pub(super) fn without_a_store_the_wording_is_the_one_we_had_before() {
        // No store is not a broken state: `middle_out` is still used for prose
        // nobody will recall verbatim, and that path must not promise a handle.
        let long: String = (0..5_000).map(|i| format!("line {i}\n")).collect();
        let out = middle_out(&long, 400);
        assert!(out.contains("re-read the source"), "{out:?}");
        assert!(!out.contains("recall_output("), "must not offer a handle it cannot honour");
    }

    #[test]
    pub(super) fn a_result_that_fits_is_untouched_and_stores_nothing() {
        let store = crate::core::observations::ObservationStore::for_session(&obs_scratch(), "s");
        let short = "well under the limit";
        assert_eq!(middle_out_recorded(short, 400, Some(&store)), short);
        assert!(std::fs::read_dir(store.dir()).map(|d| d.count()).unwrap_or(0) == 0);
    }

    #[test]
    pub(super) fn truncating_a_tool_result_records_it() {
        let store = crate::core::observations::ObservationStore::for_session(&obs_scratch(), "s");
        let long: String = (0..40_000).map(|i| format!("row {i}\n")).collect();
        let mut msgs = vec![
            json!({"role":"user","content":"go"}),
            json!({"role":"tool","content": long.clone()}),
            json!({"role":"assistant","content":"thinking"}),
        ];
        let n = truncate_tool_results(&mut msgs, 2, TOOL_RESULT_CHAR_LIMIT, Some(&store));
        assert_eq!(n, 1);
        let rewritten = msgs[1]["content"].as_str().unwrap();
        let id = rewritten.split("recall_output(id=\"").nth(1).and_then(|r| r.split('"').next()).unwrap();
        assert_eq!(store.get(id).unwrap(), long);
    }

    pub(super) fn handle_in(s: &str) -> String {
        s.split("recall_output(id=\"").nth(1).and_then(|r| r.split('"').next()).expect("a handle").to_string()
    }

    pub(super) fn build_log(lines: usize) -> String {
        (0..lines).map(|i| format!("build line {i}\n")).collect()
    }

    /// A cut is the limit plus its notice, so it is always a little over
    /// the limit, and every later pass at that limit used to cut it again and store the cut as
    /// if it were the output. Now it is left as it is, and one copy is kept: the output's.
    #[test]
    pub(super) fn a_cut_result_is_left_alone_at_the_limit_it_was_cut_to() {
        let store = crate::core::observations::ObservationStore::for_session(&obs_scratch(), "s");
        let long = build_log(20_000);
        let mut msgs = vec![json!({"role": "tool", "content": long.clone()}), json!({"role": "assistant", "content": "next"})];
        assert_eq!(truncate_tool_results(&mut msgs, 1, TOOL_RESULT_CHAR_LIMIT, Some(&store)), 1);
        let once = msgs[0]["content"].clone();
        for _ in 0..5 {
            assert_eq!(truncate_tool_results(&mut msgs, 1, TOOL_RESULT_CHAR_LIMIT, Some(&store)), 0, "nothing changed, so nothing counted");
        }
        assert_eq!(msgs[0]["content"], once);
        assert_eq!(std::fs::read_dir(store.dir()).unwrap().count(), 1, "one copy, the output's");
        assert_eq!(store.get(&handle_in(once.as_str().unwrap())).unwrap(), long);
    }

    /// A smaller limit cuts around the first cut: it keeps what cutting the output itself would
    /// keep, and the notice still names the output's handle and length.
    #[test]
    pub(super) fn a_smaller_limit_cuts_around_the_first_cut_and_keeps_its_handle() {
        let store = crate::core::observations::ObservationStore::for_session(&obs_scratch(), "s");
        let long = build_log(20_000);
        let total = long.chars().count();
        let first = middle_out_recorded(&long, TOOL_RESULT_CHAR_LIMIT, Some(&store));
        let second = middle_out_recorded(&first, 5_000, Some(&store));
        assert_eq!(handle_in(&second), handle_in(&first));
        assert!(second.contains(&format!("… [{} characters truncated", total - 5_000)), "counted from the output: {second:.300}");
        assert!(second.contains(&format!("for the whole {total}-character result")));
        let (head, rest) = second.split_once("\n\n… [").unwrap();
        let tail = rest.split_once("] …\n\n").unwrap().1;
        assert_eq!(head, long.chars().take(2_000).collect::<String>());
        assert_eq!(tail, long.chars().skip(total - 3_000).collect::<String>());
        assert_eq!(std::fs::read_dir(store.dir()).unwrap().count(), 1);
    }

    /// The turn runs `relieve_pressure` every step. Ten steps on a result that fits the budget
    /// once cut: one copy stored, and the notice still names the output.
    #[test]
    pub(super) fn ten_steps_of_pressure_relief_keep_one_copy_and_the_first_handle() {
        let store = crate::core::observations::ObservationStore::for_session(&obs_scratch(), "s");
        let long = build_log(20_000);
        let total = long.chars().count();
        let mut msgs = vec![
            msg("user", "go"),
            json!({"role": "assistant", "content": "", "tool_calls": [{"id": "c", "function": {"name": "run_shell"}}]}),
            json!({"role": "tool", "content": long.clone()}),
            json!({"role": "assistant", "content": "the next step"}),
        ];
        for _ in 0..10 {
            relieve_pressure(&mut msgs, 3, 6_000, Some(&store));
        }
        let kept = msgs[2]["content"].as_str().unwrap();
        assert!(kept.contains(&format!("for the whole {total}-character result")), "{kept:.300}");
        assert_eq!(store.get(&handle_in(kept)).unwrap(), long);
        assert_eq!(std::fs::read_dir(store.dir()).unwrap().count(), 1);
    }

    /// Without a store the same rule holds: a cut cut again still counts from the output.
    #[test]
    pub(super) fn without_a_store_a_cut_cut_again_still_counts_from_the_output() {
        let long = "q".repeat(50_000);
        let first = middle_out(&long, TOOL_RESULT_CHAR_LIMIT);
        assert_eq!(middle_out(&first, TOOL_RESULT_CHAR_LIMIT), first, "the same limit changes nothing");
        let second = middle_out(&first, 4_000);
        assert!(second.contains("… [46000 characters truncated to save context — re-read the source"), "{second:.200}");
        assert_eq!(second.matches('q').count(), 4_000);
    }
