use super::*;

    fn scratch() -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(0);
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let p = std::env::temp_dir().join(format!("su-sub-{}-{}", nanos, N.fetch_add(1, Ordering::SeqCst)));
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn an_unreadable_answer_means_do_nothing() {
        // The safety property: this loop runs unattended, so anything the model
        // says that we can't confidently read must fall to the inert branch.
        assert_eq!(Decision::parse("skip — nothing relevant"), Decision::Skip);
        assert_eq!(Decision::parse("ACT\nthere are two customer emails"), Decision::Act);
        assert_eq!(Decision::parse("- escalate\nneeds a reply"), Decision::Escalate);
        assert_eq!(Decision::parse("I'm not sure what you mean"), Decision::Skip);
        assert_eq!(Decision::parse(""), Decision::Skip);
        // "actually" starts with "act" but is not a decision… and still fails safe
        // in the only direction that matters: it never becomes escalate.
        assert_ne!(Decision::parse("hmm, actually"), Decision::Escalate);
    }

    #[test]
    fn the_reason_survives_however_the_model_lays_it_out() {
        // All three shapes a small local model actually produces for this prompt.
        // Checked against qwen2.5:7b, which answers with the bare word — the case
        // that used to leave every activity line with an empty explanation.
        assert_eq!(Decision::read("skip").1, "");
        assert_eq!(Decision::read("skip\nnothing from a customer here").1, "nothing from a customer here");
        assert_eq!(Decision::read("skip — nothing from a customer here").1, "nothing from a customer here");
        assert_eq!(Decision::read("Act: two urgent emails").1, "two urgent emails");
        assert_eq!(Decision::read("escalate\n\nneeds a reply today").1, "needs a reply today");
        // The verdict still parses in every one of those shapes.
        assert_eq!(Decision::read("Act: two urgent emails").0, Decision::Act);
        assert_eq!(Decision::read("escalate — needs judgement").0, Decision::Escalate);
        // An answer we couldn't read is inert, and says so rather than vanishing.
        let (d, why) = Decision::read("I'm not sure what you mean");
        assert_eq!(d, Decision::Skip);
        assert_eq!(why, "I'm not sure what you mean");
    }

    #[test]
    fn watches_persist_and_can_be_disabled() {
        let d = scratch();
        let mut s = Subconscious::load(&d);
        let w = s.create(Watch::new("anything urgent in my inbox?".into(), vec!["gmail".into()], false));
        assert_eq!(Subconscious::load(&d).watches.len(), 1);
        assert!(s.set_enabled(&w.id, false));
        assert!(!Subconscious::load(&d).watches[0].enabled);
        assert!(s.delete(&w.id));
        assert!(Subconscious::load(&d).watches.is_empty());
        fs::remove_dir_all(&d).ok();
    }

    #[tokio::test]
    async fn a_quiet_tick_costs_nothing_and_says_nothing() {
        // No changes anywhere → no model call, and no activity line either. A log
        // full of "nothing happened" is a log nobody reads.
        let d = scratch();
        let memory = crate::core::memory::MemoryEngine::new(d.join("vault"));
        let mut s = Subconscious::load(&d);
        s.create(Watch::new("anything urgent?".into(), vec!["gmail".into()], false));

        let r = tick(&unreachable_model(), &d, &memory).await;
        assert_eq!(r.evaluated, 0);
        assert_eq!(r.skipped_quiet, 1);
        assert!(Subconscious::load(&d).activity().is_empty());
        fs::remove_dir_all(&d).ok();
    }

    /// Settings whose model provably cannot be reached: local-only (so there is no
    /// cloud fallback) pointed at a closed port. Making the failure explicit rather
    /// than assuming "no Ollama in CI" — on a developer's machine there usually is
    /// one, and a test that quietly stops testing anything is worse than no test.
    fn unreachable_model() -> Settings {
        Settings { local_only: true, ollama_url: "http://127.0.0.1:1".into(), ..Settings::default() }
    }

    #[tokio::test]
    async fn a_tick_whose_model_is_down_leaves_the_delta_for_next_time() {
        let d = scratch();
        let memory = crate::core::memory::MemoryEngine::new(d.join("vault"));
        memory
            .save(&crate::core::memory::MemoryInput {
                filename: Some("gmail-1.md".into()),
                title: "Invoice from Acme".into(),
                body: "b".into(),
                ..Default::default()
            })
            .unwrap();
        Ledger::new(&memory.vault_path).snapshot("gmail", "test").unwrap();

        let mut s = Subconscious::load(&d);
        s.create(Watch::new("anything about invoices?".into(), vec!["gmail".into()], false));

        let r = tick(&unreachable_model(), &d, &memory).await;
        assert_eq!(r.evaluated, 1);
        assert_eq!(r.failed, 1);

        // It said so in the log…
        let log = Subconscious::load(&d).activity();
        assert_eq!(log.len(), 1);
        assert!(log[0].reason.starts_with("couldn't evaluate"), "got: {}", log[0].reason);

        // …and, crucially, the email is still waiting to be looked at.
        let still = Ledger::new(&memory.vault_path).changes_since_read("gmail", false).unwrap();
        assert_eq!(still.added.len(), 1, "a failed tick must not consume the delta");
        fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn escalating_asks_the_user_instead_of_acting() {
        // The loop reads content written by strangers. Everything it might do about
        // that ends at a card, never at a tool call.
        let d = scratch();
        let w = Watch::new("anything needing a reply?".into(), vec!["gmail".into()], true);
        let id = escalate(&d, &w, "gmail: 2 new", "two customer emails look urgent");

        let pending = crate::core::inbox::Inbox::load(&d).pending();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, id);
        assert_eq!(pending[0].tool, "subconscious_escalation");
        assert_eq!(pending[0].target, "gmail");
        assert!(pending[0].summary.contains("anything needing a reply?"));
        assert!(pending[0].summary.contains("two customer emails look urgent"));
        // Traceable back to the watch rather than posing as a conversation.
        assert!(pending[0].session_id.starts_with("__watch__"));
        fs::remove_dir_all(&d).ok();
    }

    #[tokio::test]
    async fn a_disabled_watch_is_not_evaluated() {
        let d = scratch();
        let memory = crate::core::memory::MemoryEngine::new(d.join("vault"));
        memory
            .save(&crate::core::memory::MemoryInput {
                filename: Some("gmail-1.md".into()),
                title: "Something".into(),
                body: "b".into(),
                ..Default::default()
            })
            .unwrap();
        Ledger::new(&memory.vault_path).snapshot("gmail", "test").unwrap();

        let mut s = Subconscious::load(&d);
        let w = s.create(Watch::new("anything?".into(), vec!["gmail".into()], false));
        s.set_enabled(&w.id, false);

        let r = tick(&unreachable_model(), &d, &memory).await;
        assert_eq!(r.evaluated, 0);
        assert_eq!(r.skipped_quiet, 0);
        fs::remove_dir_all(&d).ok();
    }
