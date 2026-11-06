use super::*;

    #[test]
    pub(super) fn a_user_made_assistant_can_be_deleted_and_a_builtin_cannot() {
        let d = scratch();
        std::fs::create_dir_all(d.join("personas")).unwrap();
        std::fs::write(d.join("personas/fin.md"), "---\nid: fin\nname: Fin\n---\nbody").unwrap();
        std::fs::create_dir_all(d.join("personas/fin-files")).unwrap();

        let mut r = PersonaRegistry::load(&d);
        assert!(r.manifest("fin").is_some());
        r.uninstall("fin").unwrap();
        assert!(r.manifest("fin").is_none());
        assert!(!d.join("personas/fin.md").exists(), "the manifest goes");
        assert!(!d.join("personas/fin-files").exists(), "and what it bundled");
        assert!(PersonaRegistry::load(&d).manifest("fin").is_none(), "and it stays gone");

        // Refused for a built-in: its manifest is compiled in, so "deleting" it would
        // change nothing visible and the assistant would be back on the next load.
        let err = r.uninstall("lead").unwrap_err();
        assert!(err.contains("built in"), "got: {err}");
        assert!(PersonaRegistry::load(&d).manifest("lead").is_some());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    pub(super) fn a_users_mode_choice_overrides_the_shipped_manifest_and_survives_a_reload() {
        // A built-in's manifest is compiled in, so this is the only way someone can
        // change how far a shipped assistant may act — and the page that displays
        // the mode was titled "Configure" while offering no way to change it.
        let d = scratch();
        let mut r = PersonaRegistry::load(&d);
        assert_eq!(r.manifest("lead").unwrap().default_permission_mode, "interactive");

        r.set_mode("lead", "plan").unwrap();
        assert_eq!(r.manifest("lead").unwrap().default_permission_mode, "plan");
        assert_eq!(
            PersonaRegistry::load(&d).manifest("lead").unwrap().default_permission_mode,
            "plan",
            "the choice has to outlive the process that made it"
        );

        assert!(r.set_mode("lead", "whenever-it-feels-right").is_err());
        assert!(r.set_mode("no-such-assistant", "plan").is_err());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    pub(super) fn a_roster_can_be_changed_and_cleared_back_to_the_default() {
        let d = scratch();
        let mut r = PersonaRegistry::load(&d);
        assert_eq!(r.manifest("lead").unwrap().subagents, vec!["general-purpose", "verify"]);

        r.set_roster("lead", vec!["verify".into()]).unwrap();
        assert_eq!(PersonaRegistry::load(&d).manifest("lead").unwrap().subagents, vec!["verify"]);

        // Empty is not a broken state: it is "whatever is installed", which is what
        // never having set one means, so clearing is how you go back.
        r.set_roster("lead", vec![]).unwrap();
        assert!(PersonaRegistry::load(&d).manifest("lead").unwrap().subagents.is_empty());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    pub(super) fn a_built_in_assistant_can_be_given_a_browser_and_have_it_taken_back() {
        let d = scratch();
        let mut r = PersonaRegistry::load(&d);
        assert!(!r.manifest(DEFAULT_PERSONA_ID).unwrap().tools.iter().any(|t| t == "browser"), "nobody has one until given it");
        r.set_browser(DEFAULT_PERSONA_ID, true).unwrap();
        r.set_browser(DEFAULT_PERSONA_ID, true).unwrap();
        let tools = PersonaRegistry::load(&d).manifest(DEFAULT_PERSONA_ID).unwrap().tools.clone();
        assert_eq!(tools.iter().filter(|t| *t == "browser").count(), 1, "given once, however often it is asked");
        r.set_browser(DEFAULT_PERSONA_ID, false).unwrap();
        assert!(!PersonaRegistry::load(&d).manifest(DEFAULT_PERSONA_ID).unwrap().tools.iter().any(|t| t == "browser"));
        assert!(r.set_browser("no-such-assistant", true).is_err());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    pub(super) fn who_an_assistant_may_ask_is_a_grant_that_keeps_only_real_assistants() {
        let d = scratch();
        let mut r = PersonaRegistry::load(&d);
        assert!(r.asks(DEFAULT_PERSONA_ID).contains(&"coder".to_string()), "everyone, until the person narrows it");
        r.set_asks(DEFAULT_PERSONA_ID, vec!["coder".into(), "  ".into(), "no-such-assistant".into(), DEFAULT_PERSONA_ID.into(), "coder".into()])
            .unwrap();
        assert_eq!(PersonaRegistry::load(&d).asks(DEFAULT_PERSONA_ID), vec!["coder".to_string()], "itself, repeats and strangers drop out");
        r.set_asks(DEFAULT_PERSONA_ID, vec![]).unwrap();
        assert!(PersonaRegistry::load(&d).asks(DEFAULT_PERSONA_ID).is_empty(), "clearing it is how a grant is taken back");
        assert!(r.set_asks("no-such-assistant", vec![]).is_err());

        // Where it runs: here, until it is pointed somewhere the floor allows.
        assert_eq!(r.remote(DEFAULT_PERSONA_ID), None);
        let refused = r.set_remote(DEFAULT_PERSONA_ID, "http://agent.example.com/run").unwrap_err();
        assert!(refused.starts_with("Not saved:") && refused.contains("A remote agent has to be https"), "{refused}");
        assert_eq!(PersonaRegistry::load(&d).remote(DEFAULT_PERSONA_ID), None, "a refused address is not kept");
        r.set_remote(DEFAULT_PERSONA_ID, " https://agent.example.com/run ").unwrap();
        assert_eq!(PersonaRegistry::load(&d).remote(DEFAULT_PERSONA_ID).as_deref(), Some("https://agent.example.com/run"));
        r.set_remote(DEFAULT_PERSONA_ID, "").unwrap();
        assert_eq!(PersonaRegistry::load(&d).remote(DEFAULT_PERSONA_ID), None, "an empty address brings it back here");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    pub(super) fn a_lead_can_hand_down_everything_its_workers_need() {
        // This test used to assert the opposite — that a lead has NO shell, so it
        // cannot build what it is meant to verify. That was the wrong mechanism and
        // it broke the feature: `subagent::allowed_tools` intersects a delegate's
        // toolset with its PARENT's, so a lead without shell can only spawn workers
        // without shell, and the `verify` delegate — whose entire job is running the
        // tests — got a read-only toolset. The lead could not delegate buildable
        // work at all.
        //
        // What actually keeps a builder from grading itself is the board, and it
        // does it on identity rather than on tools: the actor is the run's session
        // id, the platform names delegates `<parent>-sub1`, `-sub2`, and
        // `Board::review` refuses a verdict from whoever moved the item to review.
        // That holds whether or not the lead owns a shell. See `core::board`.
        let lead = builtins().into_iter().find(|p| p.id == "lead").unwrap();
        assert!(lead.tools.contains(&"shell".to_string()), "or its workers get no shell either");
        assert!(lead.tools.contains(&"board".to_string()));
        assert!(lead.tools.contains(&"subagent".to_string()), "it delegates or it does nothing");
        assert!(lead.system_prompt.contains("verify"), "it has to name the verify subagent");
        // The prompt has to be honest about why it holds tools it is told not to use.
        assert!(
            lead.system_prompt.contains("handing down"),
            "a lead told it has no shell, while holding one, is a prompt that lies"
        );
    }

    #[test]
    pub(super) fn the_security_assistant_never_prints_a_secret() {
        let p = builtins().into_iter().find(|p| p.id == "security").unwrap();
        assert!(
            p.system_prompt.contains("A discovered secret's value never appears anywhere"),
            "the one rule whose breach travels furthest"
        );
    }

    #[test]
    pub(super) fn registry_toggle_persists() {
        let dir = scratch();
        let r = PersonaRegistry::load(&dir);
        assert_eq!(r.sidebar().len(), 20); // all built-ins enabled+surfaced by default
        assert!(r.sidebar().iter().any(|p| p.id == "assistant" && p.default));

        let mut r = r;
        r.set_enabled("ops", false).unwrap();
        // Reload from disk to prove the choice persisted.
        let r2 = PersonaRegistry::load(&dir);
        assert_eq!(r2.sidebar().len(), 19);
        assert!(!r2.sidebar().iter().any(|p| p.id == "ops"));

        let _ = fs::remove_dir_all(&dir);
    }
