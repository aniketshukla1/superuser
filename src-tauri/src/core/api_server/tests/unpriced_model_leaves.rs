use super::*;

    #[test]
    pub(super) fn an_unpriced_model_leaves_the_total_honest_rather_than_low() {
        // A local model is free; a model we have no rate for is UNKNOWN. If the
        // unknown one silently contributed $0 the total would read as complete while
        // being short by whatever that model actually cost.
        use crate::core::sessions::{Message, Session, TokenUsage};
        let now = chrono::Local::now();
        let mk = |model: &str| {
            let mut s = Session::new("assistant", "/tmp/ws", model);
            s.created_at = now.timestamp() as f64;
            s.updated_at = s.created_at;
            s.messages.push(Message::new("assistant", "a").with_usage(Some(TokenUsage { input: 1_000_000, output: 0, cache_read: 0, cache_write: 0, ..Default::default() })));
            s
        };
        // The unknown one must carry a real provider prefix: an id no provider claims
        // is an Ollama tag, and those are genuinely free (see `core::pricing`).
        let owned = vec![mk("claude-opus-5"), mk("anthropic:claude-not-yet-released"), mk("gemma4:12b")];
        let refs: Vec<&Session> = owned.iter().collect();
        let out = compute_stats(&refs, now);

        let by = |id: &str| out["models"].as_array().unwrap().iter()
            .find(|m| m["model"] == id).cloned().unwrap();
        assert_eq!(by("claude-opus-5")["cost_usd"], 5.0);
        assert!(by("anthropic:claude-not-yet-released")["cost_usd"].is_null(), "unknown is null, not 0");
        assert_eq!(by("gemma4:12b")["cost_usd"], 0.0, "local really is free");
        // The total is the sum of what we know, and says nothing about the rest.
        assert_eq!(out["cost_usd"], 5.0);
    }
