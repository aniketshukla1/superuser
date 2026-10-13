use super::*;

    #[test]
    pub(super) fn splits_lists_but_never_prose() {
        let bulleted = "Can you take care of these for me?\n- List recurring paid subscriptions\n- Find upcoming renewals\n- Prep for next external meeting";
        let got = split_tasks(bulleted);
        assert_eq!(got.len(), 3);
        assert_eq!(got[0], "List recurring paid subscriptions");
        assert_eq!(got[2], "Prep for next external meeting");

        // numbered, both delimiters
        assert_eq!(split_tasks("1. write the memo\n2) book the room").len(), 2);

        // prose is never split — forking a single request would be worse than not
        assert!(split_tasks("summarise the doc and then email it to Sam").is_empty());
        // a single item isn't a fan-out
        assert!(split_tasks("- just one thing").is_empty());
        // list-shaped noise is ignored
        assert!(split_tasks("- ok\n- no").is_empty());
    }

    #[test]
    pub(super) fn prune_drops_only_untouched_sessions() {
        let dir = scratch();
        let store = SessionStore::new(&dir);

        // opened but never used → pruned
        let blank = Session::new("assistant", "", "m");
        store.save(&blank);
        // has a turn → kept
        let talked = Session::new("assistant", "", "m");
        store.save(&talked);
        store.append(&talked.session_id, Message::new("user", "hi")).unwrap();
        // no turn, but produced a file → kept (real work happened)
        let made = Session::new("assistant", "", "m");
        store.save(&made);
        fs::write(store.dir(&made.session_id).join("report.md"), "x").unwrap();
        // renamed by the user → kept
        let named = Session::new("assistant", "", "m");
        store.save(&named);
        store.rename(&named.session_id, "Keep me");

        assert_eq!(store.prune_empty(), 1);
        let left: Vec<String> = store.list().into_iter().map(|s| s.session_id).collect();
        assert!(!left.contains(&blank.session_id));
        for kept in [&talked.session_id, &made.session_id, &named.session_id] {
            assert!(left.contains(kept), "{kept} should have survived");
        }
        // idempotent
        assert_eq!(store.prune_empty(), 0);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    pub(super) fn info_matches_the_gui_shape() {
        let dir = scratch();
        let store = SessionStore::new(&dir);
        let s = Session::new("assistant", "/w", "llama3.2");
        store.save(&s);
        let info = s.info();
        for key in ["session_id", "title", "workspace", "agent", "model", "mode", "updated_at", "messages", "pinned", "archived"] {
            assert!(info.get(key).is_some(), "SessionInfo needs `{key}`");
        }
        assert_eq!(info["mode"], "interactive");
        assert_eq!(info["messages"], 0);
        // updated_at is ISO-8601, which is what the sidebar formats
        assert!(info["updated_at"].as_str().unwrap().contains('T'));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    pub(super) fn ids_with_separators_cannot_escape_the_store() {
        let dir = scratch();
        let store = SessionStore::new(&dir);
        // Slack/automation ids arrive from outside; path separators must be neutralized.
        let s = store.get_or_create("../../etc/passwd", "assistant", "", "m");
        store.append(&s.session_id, Message::new("user", "x")).unwrap();
        assert!(store.dir(&s.session_id).starts_with(&store.root));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    pub(super) fn a_turn_served_from_cache_is_not_mistaken_for_no_turn_at_all() {
        // `with_usage` drops empty usage so "the provider said nothing" is never
        // stored as zeros. Before the cache fields counted towards `is_empty`, a turn
        // whose input came entirely from the cache — input 0, cache_read 15,000 —
        // looked empty and was thrown away. Those are precisely the turns the caching
        // work in HARNESS-PLAN Phase 3 exists to produce, so the measurement would
        // have gone blank exactly when it started working.
        let cached = TokenUsage { input: 0, output: 120, cache_read: 15_000, cache_write: 0, ..Default::default() };
        assert!(!cached.is_empty());
        let m = Message::new("assistant", "x").with_usage(Some(cached));
        assert!(m.usage.is_some(), "a cached turn must survive being recorded");

        // Genuinely nothing is still nothing.
        assert!(TokenUsage::default().is_empty());
        assert!(Message::new("assistant", "x").with_usage(Some(TokenUsage::default())).usage.is_none());
    }

    #[test]
    pub(super) fn usage_carries_the_model_it_was_spent_on() {
        // The GUI groups usage `byModel` and renders a per-model table with an
        // "Unknown model" row for anything that carries no id. Every row took that
        // branch, because this struct is the sidecar it reads and had no model in it
        // — a breakdown by model where the model is never known.
        //
        // Per MESSAGE, not per session: a conversation can change model mid-way, and
        // showing that is the entire point of grouping by it.
        let a = TokenUsage { input: 10, output: 5, model: Some("claude-opus-5".into()), ..Default::default() };
        let b = TokenUsage { input: 7, output: 2, model: Some("gemma4:12b".into()), ..Default::default() };
        assert_ne!(a.model, b.model, "two turns, two models, one conversation");

        // It round-trips, or a reload loses the attribution the live view had.
        let json = serde_json::to_string(&a).unwrap();
        assert!(json.contains("claude-opus-5"), "{json}");
        let back: TokenUsage = serde_json::from_str(&json).unwrap();
        assert_eq!(back.model.as_deref(), Some("claude-opus-5"));

        // And an older record without one still loads, as unknown rather than as a
        // parse failure that would take the whole transcript down with it.
        let old: TokenUsage = serde_json::from_str(r#"{"input":1,"output":2}"#).unwrap();
        assert_eq!(old.model, None);
        assert_eq!(old.total(), 3);
    }

    #[test]
    pub(super) fn total_counts_every_token_the_model_actually_saw() {
        // Anthropic reports `input_tokens` EXCLUDING cache reads and writes. Summing
        // only input + output would report a well-cached turn as nearly free of
        // tokens, which is the opposite of what happened.
        let u = TokenUsage { input: 1_000, output: 500, cache_read: 8_000, cache_write: 2_000, ..Default::default() };
        assert_eq!(u.total(), 11_500);
        assert_eq!(u.input_all(), 11_000);
    }

    #[test]
    pub(super) fn the_cache_rate_distinguishes_a_cold_cache_from_no_question() {
        // Zero and unknown are different facts, and a UI that renders both as "0%"
        // reports a broken cache on a turn that never ran.
        let cold = TokenUsage { input: 15_000, output: 100, cache_read: 0, cache_write: 15_000, ..Default::default() };
        assert_eq!(cold.cache_hit_rate(), Some(0.0));
        let warm = TokenUsage { input: 0, output: 100, cache_read: 15_000, cache_write: 0, ..Default::default() };
        assert_eq!(warm.cache_hit_rate(), Some(1.0));
        assert_eq!(TokenUsage::default().cache_hit_rate(), None);

        // A provider with no prompt cache at all — Ollama, and every local runner —
        // reports neither counter. That is not a 0% hit rate; it is a question that
        // does not apply, and showing it as 0% puts a number on the dashboard that
        // no amount of caching work could ever move.
        let no_cache = TokenUsage { input: 478_827, output: 34_137, cache_read: 0, cache_write: 0, ..Default::default() };
        assert_eq!(no_cache.cache_hit_rate(), None);
    }
