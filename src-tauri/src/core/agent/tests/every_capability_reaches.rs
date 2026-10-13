use super::*;

    #[test]
    pub(super) fn an_assistant_denied_files_is_not_handed_documents_instead() {
        // Folding these into `files` must not become a way around a deliberate
        // decision to give a persona no write access to the workspace.
        let ctx = AgentContext {
            workspace: Some(std::path::PathBuf::from("/tmp/ws")),
            roots: vec![std::path::PathBuf::from("/tmp/ws")],
            has_executor: false,
            has_todo: false,
        };
        let ids: Vec<String> = vec!["web".into(), "recall_memory".into()];
        let names: Vec<String> = expand(&ids, &ctx).into_iter().map(|t| t.name).collect();
        assert!(!names.iter().any(|n| n.starts_with("sheet_")), "documents leaked to a persona with no file access");
    }

    /// Every capability we build must reach somebody.
    ///
    /// The spreadsheet bug was one instance of a shape: a capability exists, is
    /// wired, is tested — and no persona is given it, so in the running product it
    /// is dead. This walks the catalogue against both routes an assistant can get a
    /// tool by (a built-in persona, or the create-assistant allowlist) and names
    /// anything nothing can reach.
    #[test]
    pub(super) fn every_capability_reaches_at_least_one_assistant() {
        // The built-ins, as shipped.
        let builtin: Vec<Vec<&str>> = vec![
            vec!["recall_memory", "knowledge", "web", "files", "todo", "connectors", "schedule", "subagent"],
            vec!["files", "search", "shell", "git", "todo"],
            vec!["files", "search", "shell", "todo", "connectors"],
        ];
        // What the create-assistant API will accept (api_server::persona_create).
        let grantable = crate::core::personas::GRANTABLE_TOOLS;

        let orphans: Vec<&str> = catalog()
            .iter()
            .map(|c| c.id)
            .filter(|id| {
                !ALWAYS_ON.contains(id)
                    && !grantable.contains(id)
                    && !builtin.iter().any(|p| p.contains(id))
            })
            .collect();
        assert!(
            orphans.is_empty(),
            "these capabilities are built but reach nobody — no built-in persona has them and \
             the create-assistant API will not accept them: {orphans:?}"
        );
    }

    #[test]
    pub(super) fn every_assistant_can_ask_a_question_and_propose_a_plan() {
        // Including one given no tools at all: an assistant that cannot ask has to
        // guess, and guessing is the failure users actually feel.
        let ctx = AgentContext { workspace: None, roots: vec![], has_executor: false, has_todo: false };
        let names: Vec<String> = expand(&[], &ctx).into_iter().map(|t| t.name).collect();
        assert!(names.contains(&"ask_user".to_string()), "no assistant can ask a question");
        assert!(names.contains(&"propose_plan".to_string()), "no assistant can propose a plan");
    }

    #[test]
    pub(super) fn the_baseline_is_not_duplicated_when_a_persona_also_lists_it() {
        let ctx = AgentContext { workspace: None, roots: vec![], has_executor: false, has_todo: false };
        let ids: Vec<String> = vec!["ask".into(), "plan".into()];
        let names: Vec<String> = expand(&ids, &ctx).into_iter().map(|t| t.name).collect();
        assert_eq!(names.iter().filter(|n| *n == "ask_user").count(), 1, "ask_user offered twice");
    }

    #[test]
    pub(super) fn catalog_matches_known_tools() {
        let ids: Vec<&str> = catalog().iter().map(|c| c.id).collect();
        for k in KNOWN_TOOLS {
            assert!(ids.contains(k), "capability '{k}' declared in KNOWN_TOOLS but missing from catalog");
        }
        for id in &ids {
            assert!(KNOWN_TOOLS.contains(id), "capability '{id}' in catalog but not in KNOWN_TOOLS");
        }
    }

    #[test]
    pub(super) fn expand_respects_context() {
        let ids: Vec<String> = ["files", "search", "shell", "git", "todo"].iter().map(|s| s.to_string()).collect();

        // Full context → every tool present.
        let full = AgentContext { workspace: Some("/tmp".into()), roots: vec![], has_executor: true, has_todo: true };
        let names: Vec<String> = expand(&ids, &full).into_iter().map(|t| t.name).collect();
        assert!(names.contains(&"read_file".to_string()));
        assert!(names.contains(&"grep".to_string()));
        assert!(names.contains(&"run_shell".to_string()));
        assert!(names.contains(&"git_status".to_string()));
        assert!(names.contains(&"todo_write".to_string()));

        // No executor → shell is skipped; no workspace → files/search/git are skipped.
        let bare = AgentContext { workspace: None, roots: vec![], has_executor: false, has_todo: true };
        let bare_names: Vec<String> = expand(&ids, &bare).into_iter().map(|t| t.name).collect();
        assert!(!bare_names.contains(&"run_shell".to_string()));
        assert!(!bare_names.contains(&"read_file".to_string()));
        assert!(bare_names.contains(&"todo_write".to_string()));
    }

    #[test]
    pub(super) fn agent_from_manifest_and_tools() {
        let m = parse_manifest(
            "---\nid: coder\nfamily: code\ntools: [files, shell]\n---\nbe a coder",
            None,
            true,
            None,
        )
        .unwrap();
        let a = Agent::from_manifest(&m);
        assert_eq!(a.name, "coder");
        assert!(a.needs_workspace);
        let ctx = AgentContext { workspace: Some("/tmp".into()), roots: vec![], has_executor: true, has_todo: false };
        let names: Vec<String> = a.tools(&ctx).into_iter().map(|t| t.name).collect();
        assert!(names.contains(&"write_file".to_string()));
        assert!(names.contains(&"run_shell".to_string()));
    }

    #[test]
    pub(super) fn normalize_todos_coerces_status() {
        let raw = json!([
            {"content":"a","status":"completed"},
            {"content":"b","status":"weird"},
            {"content":"c","status":"in_progress"},
            "bare item"
        ]);
        let items = normalize_todos(&raw);
        assert_eq!(items[0].status, "done");
        assert_eq!(items[1].status, "pending");
        assert_eq!(items[2].status, "in_progress");
        assert_eq!(items[3].content, "bare item");
        assert_eq!(items[3].status, "pending");
    }
