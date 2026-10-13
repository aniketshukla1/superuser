use super::*;

    #[tokio::test]
    pub(super) async fn reading_synced_content_parks_a_send_that_a_rule_had_pre_approved() {
        use crate::core::memory::{MemoryInput, Origin};
        let root = scratch();
        let memory = crate::core::memory::MemoryEngine::new(root.join("vault"));
        let standing = vec!["send_message telegram:123".to_string()];

        // Clean run: the rule applies, so the send goes out (delivery itself fails
        // in a test — no bot token — but it was *attempted*, not parked).
        let ctx = ctx_for(&root, &memory, "auto", standing.clone());
        let out = dispatch(&ctx, "send_message", &json!({"target":"telegram:123","text":"nightly summary"})).await;
        assert!(!out.contains("Parked for approval"), "a clean run should honour the rule; got: {out}");
        assert!(crate::core::inbox::Inbox::load(&root).pending().is_empty());

        // A connector sync writes a stranger's words into the vault.
        memory
            .save(&MemoryInput {
                title: "Invoice overdue".into(),
                category: "Gmail".into(),
                summary: "billing".into(),
                // Ordinary synced content, deliberately. Taint is not a judgement
                // about whether a memory looks hostile — it is the rule that a run
                // which has read *anything* from outside this machine has lost its
                // standing approvals. Using an obviously hostile body here would
                // make the test pass through the write-time screen instead
                // (`memory::screen`, tested separately) and leave the property this
                // test is actually about unmeasured.
                body: "Reminder: invoice 4021 is overdue. Payment terms are net 30.".into(),
                origin: Origin::External,
                key: None,
                supersedes: None,
                ..Default::default()
            })
            .unwrap();

        // Same run shape, but recall pulls that memory in first.
        let ctx = ctx_for(&root, &memory, "auto", standing);
        let recalled = dispatch(&ctx, "recall_memory", &json!({"query":"invoice overdue billing"})).await;
        assert!(recalled.contains("Invoice overdue"), "recall should have found it; got: {recalled}");
        assert!(ctx.is_tainted(), "reading an external memory must taint the run");

        let out = dispatch(&ctx, "send_message", &json!({"target":"telegram:123","text":"vault contents"})).await;
        assert!(out.contains("Parked for approval"), "tainted send should park; got: {out}");
        let pending = crate::core::inbox::Inbox::load(&root).pending();
        assert_eq!(pending.len(), 1, "exactly the tainted send is waiting on a human");
        assert_eq!(pending[0].target, "telegram:123");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    pub(super) async fn connector_changes_reports_once_and_then_stays_quiet() {
        // The behaviour a recurring task depends on: ask twice, hear about an item
        // once. Without the marker every morning brief re-reads the whole inbox.
        use crate::core::memory::{MemoryInput, Origin};
        let root = scratch();
        let memory = crate::core::memory::MemoryEngine::new(root.join("vault"));
        memory
            .save(&MemoryInput {
                filename: Some("gmail-1.md".into()),
                title: "Invoice from Acme".into(),
                category: "Gmail".into(),
                body: "the invoice body".into(),
                origin: Origin::External,
                key: None,
                supersedes: None,
                ..Default::default()
            })
            .unwrap();
        crate::core::ledger::Ledger::new(&memory.vault_path).snapshot("gmail", "test").unwrap();

        let ctx = ctx_for(&root, &memory, "interactive", vec![]);
        let first = dispatch(&ctx, "connector_changes", &json!({"connector": "gmail"})).await;
        assert!(first.contains("1 new"), "got: {first}");
        assert!(first.contains("Invoice from Acme"), "titles, not filenames: {first}");

        let second = dispatch(&ctx, "connector_changes", &json!({"connector": "gmail"})).await;
        assert!(second.contains("nothing new"), "got: {second}");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    pub(super) async fn an_offloaded_result_can_actually_be_read_back() {
        // The claim offloading makes is "nothing is lost, you just have to ask".
        // That only holds if the path it hands back is one read_file resolves — so
        // this walks the whole way round rather than trusting the string.
        let root = scratch();
        let memory = crate::core::memory::MemoryEngine::new(root.join("vault"));
        let big = format!("HEAD-MARKER\n{}\nTAIL-MARKER", "x".repeat(crate::core::artifacts::OFFLOAD_THRESHOLD));
        let pointer = crate::core::artifacts::offload(&root, "run_shell", &big).expect("should offload");

        // Pull the path back out the way a model would have to read it.
        let rel = pointer
            .split('`')
            .nth(1)
            .expect("the pointer names a path in backticks");
        assert!(rel.starts_with("outputs/"), "got: {rel}");

        let ctx = ctx_for(&root, &memory, "interactive", vec![]);
        let read = dispatch(&ctx, "read_file", &json!({ "path": rel })).await;
        assert!(read.contains("HEAD-MARKER"), "read_file couldn't open it: {}", read.chars().take(120).collect::<String>());
        assert!(read.contains("TAIL-MARKER"), "the tail truncation would have cut is present");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    pub(super) async fn peeking_at_changes_does_not_consume_them() {
        use crate::core::memory::{MemoryInput, Origin};
        let root = scratch();
        let memory = crate::core::memory::MemoryEngine::new(root.join("vault"));
        memory
            .save(&MemoryInput {
                filename: Some("slack-1.md".into()),
                title: "#ops: deploy finished".into(),
                body: "b".into(),
                origin: Origin::External,
                key: None,
                supersedes: None,
                ..Default::default()
            })
            .unwrap();
        crate::core::ledger::Ledger::new(&memory.vault_path).snapshot("slack", "test").unwrap();

        let ctx = ctx_for(&root, &memory, "interactive", vec![]);
        let peek = dispatch(&ctx, "connector_changes", &json!({"connector": "slack", "peek": true})).await;
        assert!(peek.contains("1 new"), "got: {peek}");
        let after = dispatch(&ctx, "connector_changes", &json!({"connector": "slack"})).await;
        assert!(after.contains("1 new"), "a look is not an acknowledgement: {after}");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    pub(super) async fn recalling_only_the_users_own_notes_leaves_the_run_clean() {
        use crate::core::memory::{MemoryInput, Origin};
        let root = scratch();
        let memory = crate::core::memory::MemoryEngine::new(root.join("vault"));
        memory
            .save(&MemoryInput {
                title: "How I like summaries".into(),
                summary: "preference".into(),
                body: "Keep nightly summaries to three bullets.".into(),
                origin: Origin::Internal,
                key: None,
                supersedes: None,
                ..Default::default()
            })
            .unwrap();

        let ctx = ctx_for(&root, &memory, "auto", vec!["send_message telegram:123".to_string()]);
        let out = dispatch(&ctx, "recall_memory", &json!({"query":"nightly summaries preference"})).await;
        assert!(out.contains("How I like summaries"), "got: {out}");
        assert!(!ctx.is_tainted(), "the user's own note must not cost the run its pre-approvals");

        let _ = std::fs::remove_dir_all(&root);
    }
