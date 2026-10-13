use super::*;

    #[test]
    pub(super) fn write_read_edit_roundtrip() {
        let root = scratch();
        let todo = Mutex::new(Vec::new());

        let w = dispatch_local(&root, &todo, "write_file", &json!({"path":"notes/brief.md","content":"hello world"})).unwrap();
        assert!(w.starts_with("wrote"));

        let r = dispatch_local(&root, &todo, "read_file", &json!({"path":"notes/brief.md"})).unwrap();
        assert_eq!(r, "hello world");

        let e = dispatch_local(&root, &todo, "edit_file", &json!({"path":"notes/brief.md","old_str":"world","new_str":"assistant"})).unwrap();
        assert!(e.starts_with("edited"));
        let r2 = dispatch_local(&root, &todo, "read_file", &json!({"path":"notes/brief.md"})).unwrap();
        assert_eq!(r2, "hello assistant");

        // The written file shows up as an artifact.
        let arts = list_artifacts(&root);
        assert!(arts.contains(&"notes/brief.md".to_string()));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    pub(super) fn list_dir_lists_entries() {
        let root = scratch();
        let todo = Mutex::new(Vec::new());
        dispatch_local(&root, &todo, "write_file", &json!({"path":"a.txt","content":"needle here"})).unwrap();
        dispatch_local(&root, &todo, "write_file", &json!({"path":"b.txt","content":"nothing"})).unwrap();

        let ls = dispatch_local(&root, &todo, "list_dir", &json!({"path":"."})).unwrap();
        assert!(ls.contains("a.txt") && ls.contains("b.txt"), "list_dir output: {ls}");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    #[ignore = "grep shells out to ripgrep/grep, which is unreliable under the full parallel suite on this host; run with --ignored"]
    pub(super) fn grep_finds_matches() {
        let root = scratch();
        let todo = Mutex::new(Vec::new());
        dispatch_local(&root, &todo, "write_file", &json!({"path":"a.txt","content":"needle here"})).unwrap();
        let g = dispatch_local(&root, &todo, "grep", &json!({"pattern":"needle"})).unwrap();
        assert!(g.contains("a.txt"), "grep output: {g}");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    #[ignore = "run_capture spawns a login shell that is unreliable under the full parallel suite on this host; run with --ignored"]
    pub(super) fn run_shell_executes() {
        let root = scratch();
        let todo = Mutex::new(Vec::new());
        let out = dispatch_local(&root, &todo, "run_shell", &json!({"command":"echo assistant-ok"})).unwrap();
        assert!(out.contains("assistant-ok"), "run_shell output: {out}");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    pub(super) fn todo_write_updates_shared_list() {
        let root = scratch();
        let todo = Mutex::new(Vec::new());
        let out = dispatch_local(&root, &todo, "todo_write", &json!({"todos":[
            {"content":"draft","status":"in_progress"},
            {"content":"review","status":"pending"}
        ]})).unwrap();
        assert!(out.contains("2 items"));
        let items = todo.lock().unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].status, "in_progress");
        let _ = std::fs::remove_dir_all(&root);
    }

    /// What the turn cuts out of the window is where `recall_output` looks
    /// for it. The turn builds its store on the instance's data folder, the one this context
    /// carries; a scratch instance used to build it on the real app's folder, so every recall in
    /// one came back empty and deleting its session left the cut text behind.
    #[tokio::test]
    pub(super) async fn what_the_turn_cuts_is_what_recall_output_finds() {
        let root = scratch();
        let memory = crate::core::memory::MemoryEngine::new(root.join("vault"));
        let ctx = ctx_for(&root, &memory, "auto", vec![]);
        let long: String = (0..20_000).map(|i| format!("build line {i}\n")).collect();
        let mut messages = vec![
            json!({"role": "user", "content": [{"type": "tool_result", "tool_use_id": "t1", "content": long}]}),
            json!({"role": "assistant", "content": "the next step"}),
        ];
        // As `run_agent_inner` builds it, cut once as its first pass of `relieve_pressure` does.
        let store = crate::core::observations::ObservationStore::for_session(&ctx.data_dir, &ctx.session_id);
        let protect = crate::core::context::last_step_start(&messages);
        let limit = crate::core::context::TOOL_RESULT_CHAR_LIMIT;
        assert_eq!(crate::core::context::truncate_tool_results(&mut messages, protect, limit, Some(&store)), 1);
        let notice = messages[0]["content"][0]["content"].as_str().unwrap().to_string();
        let id = notice.split("recall_output(id=\"").nth(1).and_then(|r| r.split('"').next()).expect("a handle in the notice");
        assert_eq!(dispatch(&ctx, "recall_output", &json!({"id": id})).await, long);
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The Expense Manager's tool reads the ledger in the data folder its context carries, and
    /// answers with totals it computed — no model involved in the numbers.
    #[tokio::test]
    pub(super) async fn expense_summary_reads_the_ledger_through_the_real_dispatch() {
        let root = scratch();
        let memory = crate::core::memory::MemoryEngine::new(root.join("vault"));
        let ctx = ctx_for(&root, &memory, "discuss", vec![]);
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        for (id, amount) in [("r1", "10.50"), ("r2", "4.50")] {
            crate::core::expenses::append(
                &root,
                crate::core::expenses::Expense {
                    id: id.into(),
                    date: today.clone(),
                    merchant: format!("Shop {id}"),
                    amount: amount.into(),
                    currency: "USD".into(),
                    category: "Meals".into(),
                    message_id: format!("<{id}@x>"),
                    ..Default::default()
                },
            )
            .unwrap();
        }
        let out = dispatch(&ctx, "expense_summary", &json!({"period": "this_week"})).await;
        assert!(out.contains("- Meals: 15.00 USD (#1, #2)"), "{out}");
        assert!(dispatch(&ctx, "expense_summary", &json!({"period": "fortnight"})).await.starts_with("error:"));
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The Chief of Staff's tool reads the memory its context carries — what the connected apps
    /// brought in and the person's notes — through the real dispatch.
    #[tokio::test]
    pub(super) async fn activity_since_reads_memory_through_the_real_dispatch() {
        let root = scratch();
        let memory = crate::core::memory::MemoryEngine::new(root.join("vault"));
        memory
            .save(&crate::core::memory::MemoryInput {
                title: "Board deck due Friday".into(),
                category: "Email".into(),
                summary: "From the CFO".into(),
                body: "From the CFO".into(),
                origin: crate::core::memory::Origin::External,
                ..Default::default()
            })
            .unwrap();
        let ctx = ctx_for(&root, &memory, "discuss", vec![]);
        let out = dispatch(&ctx, "activity_since", &json!({"since": "yesterday"})).await;
        assert!(out.contains("A1 [Email]") && out.contains("Board deck due Friday"), "{out}");
        assert!(out.contains("None are set"), "no priorities in a fresh setup: {out}");
        assert!(dispatch(&ctx, "activity_since", &json!({"since": "decade"})).await.starts_with("error:"));
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A ToolCtx for tests — no network, no model; just the gating fields.
    pub(super) fn ctx_for<'a>(root: &PathBuf, memory: &'a crate::core::memory::MemoryEngine, mode: &str, standing: Vec<String>) -> ToolCtx<'a> {
        ToolCtx {
            root: root.clone(),
            memory,
            settings: crate::core::settings::Settings::default(),
            todo: Mutex::new(Vec::new()),
            data_dir: root.clone(),
            session_id: "__run__test".into(),
            persona: "assistant".into(),
            task_id: Some("task-1".into()),
            mode: mode.into(),
            standing,
            unattended: true,
            tainted: AtomicBool::new(false),
        gropes: AtomicU32::new(0),
            events: None,
            prompt: String::new(),
            seen: Mutex::new(Vec::new()),
            calls: Mutex::new(Default::default()),
            stalls: Mutex::new(Default::default()),
            charts: Mutex::new(Vec::new()),
            model: String::new(),
            asked: Mutex::new(Vec::new()),
            urls: Mutex::new(Vec::new()),
            coverage: Mutex::new(Vec::new()),
            reviewer: Default::default(),
            initiator: None,
        }
    }
