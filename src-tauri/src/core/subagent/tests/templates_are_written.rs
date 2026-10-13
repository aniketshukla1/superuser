use super::*;

    #[test]
    pub(super) fn a_user_file_overrides_a_builtin_of_the_same_name() {
        let d = scratch();
        fs::create_dir_all(d.join("subagents")).unwrap();
        fs::write(d.join("subagents/explore.md"), "---\nname: explore\ndescription: mine\ntools: read_file\n---\nMy own explore.").unwrap();
        let store = SubagentStore::load(&d);
        let t = store.get("explore").unwrap();
        assert_eq!(t.description, "mine");
        assert!(!t.builtin);
        fs::remove_dir_all(&d).ok();
    }

    #[test]
    pub(super) fn the_templates_are_written_once_and_never_clobber_edits() {
        let d = scratch();
        ensure_templates(&d);
        let readme = d.join("subagents/README.md");
        assert!(readme.exists());
        assert!(d.join("subagents/example.md.template").exists());

        // The `.template` suffix matters: an example that loaded as a real type
        // would put a research subagent in everyone's menu.
        assert!(SubagentStore::load(&d).get("research-report").is_none());
        assert_eq!(SubagentStore::load(&d).names().len(), 3, "just the built-ins");

        fs::write(&readme, "my own notes").unwrap();
        ensure_templates(&d);
        assert_eq!(fs::read_to_string(&readme).unwrap(), "my own notes");
        fs::remove_dir_all(&d).ok();
    }

    #[test]
    pub(super) fn the_shipped_example_is_a_valid_type() {
        // It's a template users copy, so it has to actually parse once renamed.
        let t = parse_type(EXAMPLE, "example").expect("the shipped example must parse");
        assert_eq!(t.name, "research-report");
        assert_eq!(t.tools.as_ref().unwrap().len(), 3);
    }

    #[test]
    pub(super) fn malformed_type_files_are_skipped_not_fatal() {
        let d = scratch();
        fs::create_dir_all(d.join("subagents")).unwrap();
        fs::write(d.join("subagents/broken.md"), "no frontmatter at all").unwrap();
        fs::write(d.join("subagents/empty.md"), "---\nname: empty\n---\n").unwrap();
        fs::write(d.join("subagents/fine.md"), "---\nname: fine\ndescription: ok\n---\nbody").unwrap();
        let store = SubagentStore::load(&d);
        assert!(store.get("broken").is_none());
        assert!(store.get("empty").is_none(), "a type with no instructions is not a type");
        assert!(store.get("fine").is_some());
        assert!(store.get("general-purpose").is_some(), "builtins survive a bad file");
        fs::remove_dir_all(&d).ok();
    }

    #[test]
    pub(super) fn a_subagent_gets_half_the_parents_steps_with_a_floor() {
        assert_eq!(step_budget(24), 12);
        assert_eq!(step_budget(30), 15);
        // A tiny parent budget must still leave room to do anything at all.
        assert_eq!(step_budget(4), 4);
        assert_eq!(step_budget(1), 4);
    }
