use super::*;

    /// Off is installed and listed, and offered to no assistant: not in a
    /// catalogue, narrowed or not, not in the names an assistant may use, not loadable.
    #[test]
    pub(super) fn a_switched_off_skill_is_listed_and_offered_to_nobody() {
        let root = with_user_skill("offered", "greeter");
        let mut store = SkillStore::load(&root);
        assert!(store.is_on("greeter"), "an installed skill starts on");
        store.set_on("greeter", false).unwrap();

        let store = SkillStore::load(&root);
        assert!(store.names().contains(&"greeter".to_string()), "still installed, still on the Skills page");
        assert!(!store.is_on("greeter"));
        assert!(!store.catalog_text().contains("greeter"));
        assert!(!store.catalog_text_for(&["greeter".into()]).contains("greeter"));
        assert!(!store.names_for(&[]).contains(&"greeter".to_string()));
        assert!(!store.names_for(&["greeter".into()]).contains(&"greeter".to_string()));
        assert!(store.usable("greeter", &[]).is_none());
        assert!(store.get("greeter").is_some(), "the page can still show what it says");

        let mut store = store;
        store.set_on("greeter", true).unwrap();
        let store = SkillStore::load(&root);
        assert!(store.catalog_text().contains("greeter: does greeter"));
        assert!(store.usable("greeter", &[]).is_some());
        let _ = fs::remove_dir_all(&root);
    }

    /// Switching every skill off leaves no catalogue at all, so the turn doesn't offer
    /// `load_skill` for a menu that is empty. Built-ins can be switched off too.
    #[test]
    pub(super) fn with_every_skill_off_there_is_no_catalogue() {
        let root = with_user_skill("all-off", "greeter");
        let mut store = SkillStore::load(&root);
        for name in store.names() {
            store.set_on(&name, false).unwrap();
        }
        let store = SkillStore::load(&root);
        assert_eq!(store.catalog_text(), "");
        assert_eq!(store.catalog_text_for(&[]), "");
        assert!(store.names_for(&[]).is_empty());
        let _ = fs::remove_dir_all(&root);
    }

    /// A skill the assistant wasn't given can't be loaded by name either — the catalogue
    /// already hid it, and `load_skill` used to hand it over anyway.
    #[test]
    pub(super) fn a_skill_off_the_assistants_list_is_not_usable() {
        let store = SkillStore::load(std::path::Path::new("/nonexistent"));
        let list = vec!["semgrep-review".to_string()];
        assert!(store.usable("semgrep-review", &list).is_some());
        assert!(store.usable("ui-component", &list).is_none());
        assert!(store.usable("ui-component", &[]).is_some(), "an empty list is everything");
        assert!(store.usable("no-such-skill", &[]).is_none());
    }

    #[test]
    pub(super) fn switching_an_unknown_skill_is_refused_and_a_gone_one_is_forgotten() {
        let root = with_user_skill("forget", "greeter");
        let mut store = SkillStore::load(&root);
        assert!(store.set_on("no-such-skill", false).is_err());
        store.set_on("greeter", false).unwrap();
        SkillStore::forget(&root, "greeter");
        assert!(SkillStore::load(&root).is_on("greeter"), "a later skill of the same name starts on");
        let _ = fs::remove_dir_all(&root);
    }

    /// Every place a skill reaches a turn asks `usable` or a list already narrowed to the
    /// switched-on ones, so a new path can't hand over a skill that is off.
    #[test]
    pub(super) fn every_turn_path_asks_whether_a_skill_is_usable() {
        let commands = crate::test_support::module_source("commands");
        let load = &commands[commands.find("\"load_skill\" => {").expect("the load_skill arm")..];
        let load = &load[..load.find("\"spawn_subagent\"").unwrap()];
        assert!(load.contains(".usable("), "load_skill must ask `usable`:\n{load}");
        let api = crate::test_support::module_source("core/api_server");
        let forced = &api[api.find("let forced: Option<(String, String)>").expect("the forced-skill path")..];
        assert!(forced[..800].contains(".usable("), "a person's pick must ask `usable`");
        assert!(!commands.contains("{ skills.names() } else"), "the skill picker must use `names_for`");
    }
