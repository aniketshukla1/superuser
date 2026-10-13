use super::*;
    use std::path::PathBuf;

    fn scratch_dir() -> PathBuf {
        let p = std::env::temp_dir().join(format!("su-stall-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&p);
        p
    }

    fn ctx<'a>(root: &PathBuf, memory: &'a crate::core::memory::MemoryEngine) -> ToolCtx<'a> {
        ToolCtx {
            root: root.clone(),
            memory,
            settings: crate::core::settings::Settings::default(),
            todo: Mutex::new(Vec::new()),
            data_dir: root.clone(),
            session_id: "__run__stall".into(),
            persona: "assistant".into(),
            task_id: None,
            mode: "ask".into(),
            standing: Vec::new(),
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

    /// The failure counter and the stall counter answer different questions, and
    /// `note_success` must only clear the first. Clearing both is what let a 22-step
    /// turn run to the step limit with every call reporting ok.
    #[test]
    fn success_clears_the_failure_count_but_not_the_stall_count() {
        let root = scratch_dir();
        let mem = crate::core::memory::MemoryEngine::new(root.clone());
        let c = ctx(&root, &mem);

        c.note_failure("grep|x");
        assert_eq!(c.failure_count("grep|x"), 1);

        assert_eq!(c.note_result("grep|x", "no matches"), 1);
        c.note_success("grep|x");
        assert_eq!(c.failure_count("grep|x"), 0, "a success forgives the failures");
        assert_eq!(c.stalled_count("grep|x"), 1, "but not what it returned");

        assert_eq!(c.note_result("grep|x", "no matches"), 2, "same answer twice");
        c.note_success("grep|x");
        assert_eq!(c.stalled_count("grep|x"), 2, "still counted after a success");
    }

    /// The case the design turns on. Re-reading a file you just edited is the
    /// commonest legitimate repeat there is, and it must never be called a loop.
    #[test]
    fn a_changed_result_resets_the_stall_count() {
        let root = scratch_dir();
        let mem = crate::core::memory::MemoryEngine::new(root.clone());
        let c = ctx(&root, &mem);

        assert_eq!(c.note_result("read_file|a.txt", "before"), 1);
        assert_eq!(c.note_result("read_file|a.txt", "before"), 2);
        assert_eq!(
            c.note_result("read_file|a.txt", "after the edit"),
            1,
            "a different result means the call is doing work again"
        );
        assert_eq!(c.stalled_count("read_file|a.txt"), 1);
    }

    /// Two different calls do not pool their counts.
    #[test]
    fn stalls_are_tracked_per_call_not_per_turn() {
        let root = scratch_dir();
        let mem = crate::core::memory::MemoryEngine::new(root.clone());
        let c = ctx(&root, &mem);

        c.note_result("grep|x", "same");
        c.note_result("grep|x", "same");
        assert_eq!(c.stalled_count("grep|x"), 2);
        assert_eq!(c.stalled_count("list_dir|."), 0);
    }
