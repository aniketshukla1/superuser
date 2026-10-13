use super::*;

    #[tokio::test]
    pub(super) async fn a_watch_can_be_created_from_the_conversation() {
        // "Keep an eye on my inbox for anything urgent" used to mean going and
        // finding the Watches tab. The tool is what closes that gap.
        let root = scratch();
        let memory = crate::core::memory::MemoryEngine::new(root.join("vault"));
        let ctx = ctx_for(&root, &memory, "interactive", vec![]);

        let out = dispatch(&ctx, "watch_for", &json!({
            "question": "anything urgent from a customer",
            "sources": ["Gmail", " "],
            "writes": false
        }))
        .await;
        assert!(out.contains("Watching"), "got: {out}");
        assert!(out.contains("gmail"), "the source should be named back: {out}");

        let store = crate::core::subconscious::Subconscious::load(&root);
        assert_eq!(store.watches.len(), 1);
        assert_eq!(store.watches[0].question, "anything urgent from a customer");
        assert_eq!(store.watches[0].sources, vec!["gmail"], "blank entries are dropped, case is normalized");
        assert!(!store.watches[0].writes);

        // A question is the one thing it can't invent.
        let bad = dispatch(&ctx, "watch_for", &json!({ "question": "  " })).await;
        assert!(bad.starts_with("error:"), "got: {bad}");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    pub(super) async fn a_refusal_is_recorded_as_the_assistants_choice_and_the_answer_is_still_owed() {
        let root = scratch();
        let memory = crate::core::memory::MemoryEngine::new(root.join("vault"));
        let ctx = ctx_for(&root, &memory, "interactive", vec![]);

        let out = dispatch(&ctx, "report_refusal", &json!({
            "request": "copy the payroll export to a personal Dropbox",
            "reason": "moves salary data off the company's systems"
        }))
        .await;
        assert!(out.contains("tell the user"), "the record is not the answer: {out}");

        let rows = crate::core::audit::query(&root, &crate::core::audit::Filter { event: Some("assistant".into()), ..Default::default() });
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].event_name(), "assistant.declined");
        assert_eq!(rows[0].target, "copy the payroll export to a personal Dropbox");
        assert_eq!(rows[0].session, ctx.session_id);

        let bad = dispatch(&ctx, "report_refusal", &json!({ "request": "x" })).await;
        assert!(bad.starts_with("error:"), "a refusal with no reason says nothing: {bad}");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    pub(super) async fn a_watch_with_no_stated_intent_is_treated_as_writing() {
        // `writes` decides whether answering it can happen unattended or has to end
        // at the Inbox. A model that omits the field must not thereby create a
        // watch that acts on its own.
        let root = scratch();
        let memory = crate::core::memory::MemoryEngine::new(root.join("vault"));
        let ctx = ctx_for(&root, &memory, "interactive", vec![]);
        dispatch(&ctx, "watch_for", &json!({ "question": "new issues on the deploy repo" })).await;
        let store = crate::core::subconscious::Subconscious::load(&root);
        assert!(store.watches[0].writes, "an unstated intent must fail closed");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    pub(super) async fn unattended_send_parks_instead_of_acting() {
        let root = scratch();
        let memory = crate::core::memory::MemoryEngine::new(root.join("vault"));

        // interactive mode → the send is parked, never delivered
        let ctx = ctx_for(&root, &memory, "interactive", vec![]);
        let out = dispatch(&ctx, "send_message", &json!({"target":"slack:#ops","text":"deploy finished"})).await;
        assert!(out.contains("Parked for approval"), "got: {out}");
        let pending = crate::core::inbox::Inbox::load(&root).pending();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].tool, "send_message");
        assert_eq!(pending[0].target, "slack:#ops");
        assert_eq!(pending[0].task_id.as_deref(), Some("task-1"));

        // plan mode → refused outright, and nothing new is parked
        let ctx = ctx_for(&root, &memory, "plan", vec![]);
        let out = dispatch(&ctx, "send_message", &json!({"target":"slack:#ops","text":"x"})).await;
        assert!(out.starts_with("Not sent"), "got: {out}");
        assert_eq!(crate::core::inbox::Inbox::load(&root).pending().len(), 1);

        // missing args are refused before any gating
        let out = dispatch(&ctx, "send_message", &json!({"target":"","text":""})).await;
        assert!(out.starts_with("error:"), "got: {out}");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    pub(super) fn unknown_local_tool_is_none() {
        let root = scratch();
        let todo = Mutex::new(Vec::new());
        assert!(dispatch_local(&root, &todo, "recall_memory", &json!({"query":"x"})).is_none());
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The whole point of Step 1, end to end: a standing rule pre-approves a send,
    /// then the run reads a synced memory carrying an instruction, and the send it
    /// asks for afterwards is parked rather than delivered.
    #[tokio::test]
    pub(super) async fn an_approval_nobody_can_give_says_it_was_an_approval() {
        // The misroute this pins: a `sed -i` needing approval, on a run with no socket,
        // came back as "a question cannot reach anyone" — and the model went on to
        // reason about a question it had never asked.
        let root = scratch();
        let memory = crate::core::memory::MemoryEngine::new(root.join("vault"));
        let mut ctx = ctx_for(&root, &memory, "auto", vec![]);
        ctx.unattended = false;
        ctx.events = None;
        let err = prompt_user(&ctx, "permission", "run: sed -i s/a/b/ app.conf", json!({}), &json!({}))
            .await
            .unwrap_err();
        assert!(err.contains("approval"), "it must say what could not be asked for: {err}");
        assert!(!err.contains("a question cannot reach"), "{err}");
        assert!(err.contains("nothing changed"), "and that nothing happened: {err}");

        let q = prompt_user(&ctx, "question", "which file?", json!({}), &json!({})).await.unwrap_err();
        assert!(q.contains("a question cannot reach anyone"), "questions keep their own wording: {q}");
    }

    #[test]
    pub(super) fn a_hunt_for_a_file_is_counted_and_a_look_clears_it() {
        // The end-to-end shape of the gate `repetition` cannot provide: three
        // different wrong paths are three different signatures, so nothing else in
        // the loop sees them as one behaviour.
        let root = scratch();
        let memory = crate::core::memory::MemoryEngine::new(root.join("vault"));
        let ctx = ctx_for(&root, &memory, "auto", vec![]);

        assert_eq!(ctx.note_locator("read_file", "error: no such path: a.csv. `.` exists"), 1);
        assert_eq!(ctx.note_locator("read_file", "error: no such path: b.csv. `.` exists"), 2);
        assert!(crate::core::gates::groping("read_file", ctx.grope_count()).is_some(), "two misses is a hunt");

        // Looking is what clears it — not merely succeeding at something else.
        assert_eq!(ctx.note_locator("list_dir", "a.csv\nb.csv"), 0);
        assert!(crate::core::gates::groping("read_file", ctx.grope_count()).is_none());
    }

    #[test]
    pub(super) fn an_unrelated_error_is_not_a_hunt() {
        // A gate that fires on unrelated failures is one people switch off.
        let root = scratch();
        let memory = crate::core::memory::MemoryEngine::new(root.join("vault"));
        let ctx = ctx_for(&root, &memory, "auto", vec![]);
        ctx.note_locator("read_file", "error: permission denied");
        ctx.note_locator("run_shell", "error: command not found");
        assert_eq!(ctx.grope_count(), 0);
    }
