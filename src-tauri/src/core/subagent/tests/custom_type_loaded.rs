use super::*;

    /// A parent with every capability, so the filtering is tested against the
    /// widest toolset a subagent could ever inherit.
    pub(super) fn parent_tools() -> Vec<ToolSpec> {
        let ids: Vec<String> = crate::core::personas::KNOWN_TOOLS.iter().map(|s| s.to_string()).collect();
        expand(
            &ids,
            &AgentContext { workspace: Some(std::env::temp_dir()), roots: vec![], has_executor: true, has_todo: true },
        )
    }

    pub(super) fn scratch() -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(0);
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let p = std::env::temp_dir().join(format!("bba-sub-{}-{}", nanos, N.fetch_add(1, Ordering::SeqCst)));
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    pub(super) fn the_builtins_are_always_available() {
        let d = scratch();
        let s = SubagentStore::load(&d);
        assert!(s.get("general-purpose").is_some());
        assert!(s.get("explore").is_some());
        assert!(s.get("verify").is_some());
        assert!(s.get("nonsense").is_none());
        assert!(s.catalog_text().contains("Read-only investigation"));
        fs::remove_dir_all(&d).ok();
    }

    #[test]
    pub(super) fn no_subagent_can_message_drive_the_desktop_or_recurse() {
        let parent = parent_tools();
        assert!(parent.iter().any(|t| t.name == "send_message"), "the parent has it to begin with");

        // Even the type that inherits everything.
        let general = SubagentType { name: "g".into(), description: String::new(), tools: None, instructions: String::new(), builtin: true };
        let got = allowed_tools(&general, &parent);
        for blocked in BLOCKED {
            assert!(!got.iter().any(|t| &t.name == blocked), "{blocked} must never reach a subagent");
        }
        assert!(got.iter().any(|t| t.name == "read_file"), "ordinary tools still come through");
        assert!(got.iter().any(|t| t.name == "run_shell"), "general-purpose keeps shell");
    }

    /// The Team Lead delegates everything, so whatever its delegates can do is
    /// bounded by what IT was granted. This is where that bites.
    #[test]
    pub(super) fn a_roster_narrows_the_menu_to_the_team() {
        let d = scratch();
        let s = SubagentStore::load(&d);
        let all = s.catalog_text();
        assert!(all.contains("general-purpose") && all.contains("verify") && all.contains("explore"));

        let team = s.catalog_text_for(&["verify".to_string()]);
        assert!(team.contains("verify"));
        assert!(!team.contains("explore"), "a roster is a list, not a suggestion:\n{team}");
        fs::remove_dir_all(&d).ok();
    }

    #[test]
    pub(super) fn a_roster_naming_nothing_installed_narrows_to_nothing() {
        // Not to everything. Quietly widening a deliberate list is how a lead ends
        // up delegating to a worker nobody vetted.
        let d = scratch();
        let s = SubagentStore::load(&d);
        assert_eq!(s.catalog_text_for(&["ghost-worker".to_string()]), "");
        assert_eq!(s.missing_from(&["ghost-worker".to_string(), "verify".to_string()]), vec!["ghost-worker".to_string()]);
        fs::remove_dir_all(&d).ok();
    }

    #[test]
    pub(super) fn the_leads_declared_team_actually_exists() {
        // A roster naming a type nobody ships means the lead has no one to staff,
        // and the failure is silent at runtime.
        let d = scratch();
        let s = SubagentStore::load(&d);
        let registry = crate::core::personas::PersonaRegistry::load(&d);
        let lead = registry.manifest("lead").expect("the Team Lead ships");
        assert!(!lead.subagents.is_empty(), "a lead with no roster staffs whatever is lying around");
        assert!(
            s.missing_from(&lead.subagents).is_empty(),
            "the Team Lead names worker types that do not exist: {:?}",
            s.missing_from(&lead.subagents)
        );
        fs::remove_dir_all(&d).ok();
    }

    #[test]
    pub(super) fn a_delegate_can_only_ever_have_what_its_parent_has() {
        let d = scratch();
        let s = SubagentStore::load(&d);
        // The REAL Team Lead grant, read from the shipped manifest rather than
        // copied here — a copy would keep passing after someone narrowed the
        // persona, which is the exact change this test exists to catch.
        let registry = crate::core::personas::PersonaRegistry::load(&d);
        let manifest = registry.manifest("lead").expect("the Team Lead ships");
        let lead = expand(
            &manifest.tools,
            &AgentContext { workspace: Some(std::env::temp_dir()), roots: vec![], has_executor: true, has_todo: true },
        );
        let verifier = allowed_tools(s.get("verify").unwrap(), &lead);
        let names: Vec<&str> = verifier.iter().map(|t| t.name.as_str()).collect();
        assert!(
            names.contains(&"run_shell"),
            "a verifier that cannot run the tests cannot verify anything — it has only {names:?}"
        );
        fs::remove_dir_all(&d).ok();
    }

    #[test]
    pub(super) fn a_verifier_cannot_edit_the_work_it_is_judging() {
        // A reviewer that can edit the work is a reviewer that can make it pass.
        let d = scratch();
        let s = SubagentStore::load(&d);
        let tools = s.get("verify").unwrap().tools.clone().unwrap();
        for banned in ["write_file", "edit_file", "apply_patch", "board_move"] {
            assert!(!tools.contains(&banned.to_string()), "a verifier must not have `{banned}`");
        }
        // But it must be able to actually check, and to record what it found.
        for needed in ["run_shell", "read_file", "board_review"] {
            assert!(tools.contains(&needed.to_string()), "a verifier needs `{needed}`");
        }
        fs::remove_dir_all(&d).ok();
    }

    #[test]
    pub(super) fn explore_cannot_change_anything() {
        let parent = parent_tools();
        let store = SubagentStore::load(&scratch());
        let got = allowed_tools(store.get("explore").unwrap(), &parent);
        for writer in ["write_file", "edit_file", "run_shell", "connector_sync", "send_message"] {
            assert!(!got.iter().any(|t| t.name == writer), "explore must not have {writer}");
        }
        assert!(got.iter().any(|t| t.name == "read_file"));
        assert!(got.iter().any(|t| t.name == "web_search"));
    }

    #[test]
    pub(super) fn a_custom_type_is_loaded_and_its_allowlist_is_exact() {
        let d = scratch();
        fs::create_dir_all(d.join("subagents")).unwrap();
        fs::write(
            d.join("subagents/research.md"),
            "---\nname: research\ndescription: Research one topic and report with citations\ntools: web_search, web_fetch, read_file\n---\nYou research one topic and return one report.",
        )
        .unwrap();

        let store = SubagentStore::load(&d);
        let t = store.get("research").expect("custom type loads");
        assert!(!t.builtin);
        assert!(t.instructions.starts_with("You research"));

        let got = allowed_tools(t, &parent_tools());
        let names: Vec<&str> = got.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names.len(), 3, "exactly the listed tools: {names:?}");
        assert!(names.contains(&"web_search") && names.contains(&"read_file"));
        fs::remove_dir_all(&d).ok();
    }

    #[test]
    pub(super) fn a_custom_type_cannot_grant_itself_a_blocked_tool() {
        let t = SubagentType {
            name: "sneaky".into(),
            description: String::new(),
            tools: Some(vec!["read_file".into(), "send_message".into(), "spawn_subagent".into()]),
            instructions: "…".into(),
            builtin: false,
        };
        let names: Vec<String> = allowed_tools(&t, &parent_tools()).into_iter().map(|t| t.name).collect();
        assert_eq!(names, vec!["read_file"], "the blocklist wins over the allowlist");
    }
