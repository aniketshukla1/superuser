use super::*;

    fn scratch() -> PathBuf {
        let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let d = std::env::temp_dir().join(format!("bba-authoring-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn a_skill() -> SkillDraft {
        SkillDraft {
            name: "weekly-ops-review".into(),
            description: "Turn a week of incidents into the three things to fix".into(),
            instructions: "1. Read every incident raised in the last seven days.\n2. Group them by what actually failed, not by which service reported it.\n3. Name the three worth fixing and say what each would take.".into(),
            tools: vec!["read_file".into()],
            replace: false,
        }
    }

    #[test]
    fn a_skill_is_checked_the_way_the_form_checks_one() {
        let d = scratch();
        let store = SkillStore::load(&d);
        assert!(check_skill(&store, &a_skill()).is_empty());

        let mut bad = a_skill();
        bad.name = "Weekly Ops Review".into();
        assert!(check_skill(&store, &bad)[0].contains("isn't a usable skill name"));

        let mut thin = a_skill();
        thin.instructions = "Summarise the week.".into();
        assert!(check_skill(&store, &thin)[0].contains("a sentence, not a skill"));

        let mut mute = a_skill();
        mute.description = "  ".into();
        assert!(check_skill(&store, &mute)[0].contains("one line saying when to reach for it"));

        let mut invented = a_skill();
        invented.tools = vec!["read_file".into(), "summon_the_ops_team".into()];
        let problems = check_skill(&store, &invented);
        assert_eq!(problems.len(), 1);
        assert!(problems[0].contains("summon_the_ops_team") && problems[0].contains("list_skill_tools"));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_skill_that_already_exists_is_not_replaced_by_accident() {
        let d = scratch();
        let store = SkillStore::load(&d);
        let existing = store.names().into_iter().next().expect("built-ins exist");
        let mut draft = a_skill();
        draft.name = existing.clone();
        assert!(check_skill(&store, &draft)[0].contains("already exists"));
        draft.replace = true;
        assert!(check_skill(&store, &draft).is_empty(), "saying so is how you mean it");
        let _ = std::fs::remove_dir_all(&d);
    }

    fn a_persona() -> PersonaDraft {
        PersonaDraft {
            id: "sql-reviewer".into(),
            name: "SQL Reviewer".into(),
            tagline: "Reads migrations before they run".into(),
            instructions: "You review SQL migrations before anyone runs them.\n\nRead the migration and the schema it changes. Say what it locks, how long it holds the lock, and what happens to a query that arrives mid-migration. Refuse to approve one that rewrites a table without a plan for the lock.".into(),
            tools: vec!["files".into(), "search".into()],
            skills: vec![],
            mode: "interactive".into(),
            family: "code".into(),
        }
    }

    #[test]
    fn an_assistant_is_checked_against_the_app_that_will_load_it() {
        let d = scratch();
        let reg = PersonaRegistry::load(&d);
        let store = SkillStore::load(&d);
        assert!(check_persona(&reg, &store, &a_persona()).is_empty());

        let mut taken = a_persona();
        taken.id = "coder".into();
        assert!(check_persona(&reg, &store, &taken)[0].contains("already an assistant"));

        let mut ungrantable = a_persona();
        ungrantable.tools = vec!["files".into(), "recall".into()];
        assert!(check_persona(&reg, &store, &ungrantable)[0].contains("isn't something an assistant can be given"));

        let mut unknown_skill = a_persona();
        unknown_skill.skills = vec!["no-such-skill".into()];
        assert!(check_persona(&reg, &store, &unknown_skill)[0].contains("list_persona_skills"));

        // A mode the app doesn't have is caught by the app's own parser, not by a second
        // copy of its rules here.
        let mut odd_mode = a_persona();
        odd_mode.mode = "whenever".into();
        assert!(check_persona(&reg, &store, &odd_mode)[0].contains("didn't come out valid"));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_saved_assistant_is_one_the_registry_loads_and_has_switched_on() {
        let d = scratch();
        let mut draft = a_persona();
        // A tagline that would break hand-written YAML, on purpose.
        draft.tagline = "Reads migrations: before they run, not after".into();
        let path = save_persona(&d, &draft).unwrap();
        assert!(path.ends_with("sql-reviewer.md"));
        let reg = PersonaRegistry::load(&d);
        let m = reg.manifest("sql-reviewer").expect("the registry loads it");
        assert_eq!(m.tagline, "Reads migrations: before they run, not after");
        assert_eq!(m.tools, vec!["files".to_string(), "search".to_string()]);
        assert!(reg.is_enabled("sql-reviewer"), "the person pressed the card — that is the consent");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn the_tool_list_a_skill_may_declare_is_the_one_the_app_has() {
        let names = tool_names();
        assert!(names.contains(&"read_file".to_string()) && names.contains(&"ask_coworker".to_string()));
        assert!(names.windows(2).all(|w| w[0] <= w[1]), "sorted, so a model reads it as a list rather than a heap");
        let grouped = tools_by_capability();
        assert!(grouped.iter().any(|(c, t)| c == "files" && t.iter().any(|n| n == "read_file")));
    }
