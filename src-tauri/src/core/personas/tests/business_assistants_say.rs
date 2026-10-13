use super::*;

    #[test]
    pub(super) fn the_business_assistants_say_where_their_judgement_stops() {
        // These three are the ones whose confident wrong answer has a cost the user
        // pays somewhere else: a board pack, a tribunal, a customer's account. The
        // limit has to be in the prompt, because a model asked about redundancy pay
        // or a wire transfer will otherwise answer helpfully and plausibly.
        let by = |id: &str| builtins().into_iter().find(|p| p.id == id).unwrap();

        let finance = by("finance");
        assert!(
            finance.system_prompt.contains("do not move money"),
            "the Finance Analyst analyses; executing a payment is the user's"
        );

        let hr = by("hr");
        assert!(
            hr.system_prompt.contains("qualified advice"),
            "the HR Partner must hand employment law back rather than answering it"
        );

        let support = by("support");
        assert!(
            support.system_prompt.contains("password"),
            "the Support assistant must never collect a credential from a customer"
        );

        // These three read text that someone else wrote — a support ticket, a CV, an
        // invoice — which is the definition of untrusted input. This test used to
        // assert they had no shell, on the argument that a capability granted "just
        // in case" is one a prompt injection in a ticket or a CV can reach for. That
        // argument was right and is not withdrawn.
        //
        // They have a shell now regardless: asked for directly, twice, on 2026-09-01,
        // so the assistants can actually run things. What changes is which control
        // carries the weight. It was "the tool does not exist"; it is now the
        // approval gate, because `run_shell` is `RiskClass::Exec` and these personas
        // are `interactive` — every command is shown to the user before it runs.
        //
        // So `interactive` is no longer a nicety on these three, it is the whole
        // mitigation, and this test defends it much harder than it did before. A
        // future change that flips any of them to an unattended mode would take the
        // last thing standing between a poisoned ticket and a command, and it must
        // fail here rather than ship.
        for id in ["finance", "hr", "support"] {
            let p = by(id);
            assert_eq!(
                p.default_permission_mode, "interactive",
                "'{id}' reads untrusted text and can now run commands — the approval \
                 gate is the only thing between those two facts"
            );
            // Still no git: nothing here has a repo to be in, and a write that can
            // rewrite history is a different blast radius from one command at a time.
            assert!(!p.tools.contains(&"git".to_string()), "'{id}' has no use for git");
        }
    }

    #[test]
    pub(super) fn the_security_personas_cannot_skip_a_check_quietly() {
        // The doctrine these personas exist to carry. A security assistant that runs
        // whatever happens to be installed and reports on that alone is worse than
        // no security assistant: it produces a clean-looking answer with no coverage.
        for id in ["security", "cloud-posture", "dep-audit"] {
            let p = builtins().into_iter().find(|p| p.id == id).unwrap();
            let prompt = &p.system_prompt;
            assert!(prompt.contains("check_tool"), "'{id}' must check before it scans");
            assert!(prompt.contains("record_coverage"), "'{id}' must record a degraded check");
            assert!(prompt.contains("coverage_note"), "'{id}' must end with the coverage note");
            assert!(p.tools.contains(&"toolchain".to_string()), "'{id}' needs the toolchain capability");
            // Interactive, not auto: these run scanners over someone's real repo and
            // open branches. Nothing here should act unattended by default.
            assert_eq!(p.default_permission_mode, "interactive", "'{id}' must not default to acting alone");
        }
    }

    #[test]
    pub(super) fn assistants_saved_under_the_old_word_are_renamed_on_load() {
        // Shipped copy moved with the rename; files on disk did not, so a list of
        // built-ins reading "Assistant" sat above user-made ones reading "Your
        // Finance coworker".
        let d = scratch();
        std::fs::create_dir_all(d.join("personas")).unwrap();
        std::fs::write(
            d.join("personas/fin.md"),
            "---\nid: fin\nname: Fin\ntagline: \"Your Fin coworker\"\ndescription: \"Your Fin coworker\"\n---\nYou are Fin, the user's coworker.",
        )
        .unwrap();

        let r = PersonaRegistry::load(&d);
        let m = r.manifest("fin").expect("the assistant still loads");
        assert_eq!(m.tagline, "Your Fin assistant");
        assert!(m.system_prompt.contains("the user's assistant"), "the body too: {}", m.system_prompt);

        // On disk, not just in memory — the next process must not have to redo it.
        let on_disk = std::fs::read_to_string(d.join("personas/fin.md")).unwrap();
        assert!(!on_disk.contains("coworker"), "{on_disk}");
        assert!(on_disk.contains("id: fin"), "everything else is left alone");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    pub(super) fn the_full_list_keeps_disabled_assistants_and_the_picker_does_not() {
        // The distinction that made deleting one disabled assistant look like it
        // deleted all of them: `sidebar` is the picker's view — enabled AND surfaced
        // — while the settings tab shows everything. A handler returning the wrong
        // one replaces the client's list with a subset.
        let d = scratch();
        std::fs::create_dir_all(d.join("personas")).unwrap();
        std::fs::write(d.join("personas/fin.md"), "---\nid: fin\nname: Fin\n---\nbody").unwrap();

        let mut r = PersonaRegistry::load(&d);
        r.set_enabled("fin", false).unwrap();

        assert!(r.list_all().iter().any(|p| p.id == "fin"), "settings shows a disabled assistant");
        assert!(!r.sidebar().iter().any(|p| p.id == "fin"), "the picker does not");
        assert!(
            r.list_all().len() > r.sidebar().len(),
            "so returning the picker where the list is expected loses rows"
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    pub(super) fn a_name_already_in_use_is_a_refusal_not_a_number() {
        // The slug is the id, and the id decides everything downstream. Appending a
        // number silently is how someone types "Finance", gets an assistant called
        // `finance-2`, cannot find it under the name they chose, and tries again —
        // four times, in this app's own audit log. The handler that enforces this
        // lives in `api_server::persona_create`; what this pins is the fact it
        // checks, which is that the built-in roster already holds these slugs.
        let r = PersonaRegistry::load(std::path::Path::new("/nonexistent"));
        for taken in ["finance", "coder", "support", "hr", "assistant"] {
            assert!(
                r.manifest(taken).is_some(),
                "'{taken}' must be a name the create handler refuses — a built-in holds it"
            );
        }
        // And a name nobody holds stays available.
        assert!(r.manifest("finance-ops").is_none());
    }

    #[test]
    pub(super) fn a_persona_that_merely_arrived_is_off_until_someone_turns_it_on() {
        // Both halves of the default matter, and they pull opposite ways.
        //
        // OFF is right for a persona that ARRIVED — installed from a repo, a gallery,
        // a folder someone pointed at. Nobody has read what it does yet, and a manifest
        // that switches itself on by appearing on disk is a drop-target.
        //
        // But the create handler in `api_server` writes a manifest too, for an assistant
        // the user just typed into a modal and pressed Create on. Left at the default it
        // was invisible in its own product: absent from the picker, unresolvable by the
        // client (so the header showed the slug, not the name), and owning a session the
        // sidebar filters out — which is how creating an assistant landed you back in
        // whichever assistant you came from. That handler must call `set_enabled`.
        let d = scratch();
        std::fs::create_dir_all(d.join("personas")).unwrap();
        std::fs::write(d.join("personas/arrived.md"), "---\nid: arrived\nname: Arrived\n---\nbody").unwrap();

        let mut r = PersonaRegistry::load(&d);
        assert!(r.manifest("arrived").is_some(), "it is installed");
        assert!(!r.is_enabled("arrived"), "and off, because nobody asked for it yet");
        assert!(!r.sidebar().iter().any(|p| p.id == "arrived"), "so it is not in the picker");

        // What the create handler does after writing the file.
        r.set_enabled("arrived", true).unwrap();
        assert!(r.sidebar().iter().any(|p| p.id == "arrived"), "now you can start a session as it");
        assert!(
            PersonaRegistry::load(&d).is_enabled("arrived"),
            "and it is still there after a restart — otherwise the assistant vanishes on relaunch"
        );
        let _ = std::fs::remove_dir_all(&d);
    }
