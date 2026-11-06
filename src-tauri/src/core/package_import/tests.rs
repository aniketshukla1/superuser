use super::*;

    fn temp(tag: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("su-obimport-{tag}-{}", super::super::inbox::short_id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    const LONG: &str = "Search the connected sources first, then read the file you found rather than answering from its title. \
                        If nothing turns up, say what you searched for. Name the document you used.";

    fn package(dir: &Path) {
        std::fs::write(
            dir.join("agents.yaml"),
            format!(
                r#"agents:
  - id: desk-helper
    name: Desk Helper
    title: Everyday Work
    role_description: Help with everyday work.
    type: built-in
    system_prompt: Give clear, short answers.
    skills: [bot-creator, find-a-doc, not-in-package]
  - id: remote-risk
    name: Remote Risk
    title: Compliance
    role_description: Check claims against the rules.
    type: remote-ag-ui
    endpoint: ${{RISK_URL:-https://risk.example.com/agui}}
  - id: unset-remote
    name: Unset Remote
    type: remote-ag-ui
    endpoint: ${{SOME_SECRET_ENV}}
  - id: Bad Id
    name: Broken
    type: built-in
    system_prompt: x
"#
            ),
        )
        .unwrap();
        std::fs::write(
            dir.join("skills.yaml"),
            format!(
                r#"skills:
  - slug: find-a-doc
    title: Find a document
    summary: Search the connected sources for a file and read it.
    instructions: >-
      {LONG}
    tools: [google-drive/search_files, local/web_fetch]
  - slug: bot-creator
    title: Make a coworker
    summary: Make one.
    instructions: whatever
  - slug: tiny
    title: Tiny
    summary: Too short.
    instructions: Do it.
"#
            ),
        )
        .unwrap();
        std::fs::write(dir.join("channels.yaml"), "channels: []\n").unwrap();
        std::fs::write(dir.join("model.yaml"), "model:\n  provider: someprovider\n").unwrap();
        std::fs::write(dir.join("knowledge.yaml"), "sources:\n  - type: google-drive\n").unwrap();
    }

    #[test]
    fn an_environment_reference_takes_its_default_and_nothing_else() {
        std::env::set_var("SU_TEST_PACKAGE_SECRET", "hunter2");
        assert_eq!(without_environment("${SU_TEST_PACKAGE_SECRET}"), "", "the environment is never read");
        assert_eq!(without_environment("${SU_TEST_PACKAGE_SECRET:-https://a.example.com}"), "https://a.example.com");
        assert_eq!(without_environment(" https://b.example.com "), "https://b.example.com");
        std::env::remove_var("SU_TEST_PACKAGE_SECRET");
    }

    #[test]
    fn a_package_is_planned_into_what_has_a_home_and_what_is_left_behind() {
        let data = temp("data");
        let pkg = temp("pkg");
        package(&pkg);
        let plan = plan(&pkg, &PersonaRegistry::load(&data), &SkillStore::load(&data)).unwrap();

        // Skills: the authoring one maps to ours, a thin one is refused with its reason, and
        // a connected app's tool reference is kept aside rather than declared.
        let find = plan.skills.iter().find(|s| s.name == "find-a-doc").unwrap();
        assert!(find.problems.is_empty(), "{:?}", find.problems);
        assert_eq!(find.tools, vec!["web_fetch"]);
        assert_eq!(find.unmapped, vec!["google-drive/search_files"]);
        assert!(plan.skills.iter().all(|s| s.name != "bot-creator"));
        assert!(plan.skills.iter().find(|s| s.name == "tiny").unwrap().problems[0].contains("too short"));

        // An ordinary bot: who it is comes first, its own prompt after, and the skill it held
        // for making coworkers becomes ours with the grant that makes it work.
        let desk = plan.personas.iter().find(|p| p.id == "desk-helper").unwrap();
        assert!(desk.problems.is_empty(), "{:?}", desk.problems);
        assert!(desk.instructions.starts_with("You are Desk Helper, Everyday Work. Help with everyday work."));
        assert!(desk.instructions.ends_with("Give clear, short answers."));
        assert_eq!(desk.skills, vec!["persona-creator", "find-a-doc"]);
        assert!(desk.tools.contains(&"authoring".to_string()));
        assert!(desk.notes.iter().any(|n| n.contains("not-in-package")), "{:?}", desk.notes);

        // A remote bot keeps its address and gets no tools of ours.
        let remote = plan.personas.iter().find(|p| p.id == "remote-risk").unwrap();
        assert_eq!(remote.remote.as_deref(), Some("https://risk.example.com/agui"));
        assert!(remote.tools.is_empty());
        // One whose address is the deployment's is left behind, and says how to bring it.
        assert!(plan.personas.iter().all(|p| p.id != "unset-remote"));
        assert!(plan.left_behind.iter().any(|l| l.starts_with("Unset Remote") && l.contains("Where it runs")));

        assert!(!plan.personas.iter().find(|p| p.name == "Broken").unwrap().problems.is_empty());
        for expected in ["Channels", "The model (someprovider)", "Document sources (google-drive)", "`bot-creator` skill"] {
            assert!(plan.left_behind.iter().any(|l| l.contains(expected)), "missing {expected}: {:?}", plan.left_behind);
        }
        let _ = std::fs::remove_dir_all(&data);
        let _ = std::fs::remove_dir_all(&pkg);
    }

    #[test]
    fn an_import_writes_what_was_read_and_switches_nothing_on() {
        let data = temp("data2");
        let pkg = temp("pkg2");
        package(&pkg);
        let preview = plan(&pkg, &PersonaRegistry::load(&data), &SkillStore::load(&data)).unwrap();

        // A folder that changed after it was read is refused.
        std::fs::write(pkg.join("model.yaml"), "model:\n  provider: other\n").unwrap();
        let refused = apply(&data, &pkg, &preview.digest).unwrap_err();
        assert!(refused.contains("changed after it was read"), "{refused}");

        let preview = plan(&pkg, &PersonaRegistry::load(&data), &SkillStore::load(&data)).unwrap();
        let applied = apply(&data, &pkg, &preview.digest).unwrap();
        assert_eq!(applied.skills, vec!["find-a-doc"]);
        let ids: Vec<&str> = applied.consent.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, vec!["desk-helper", "remote-risk"]);

        let reg = PersonaRegistry::load(&data);
        assert!(!reg.is_enabled("desk-helper") && !reg.is_enabled("remote-risk"), "switching one on is the person's consent");
        assert_eq!(reg.remote("remote-risk").as_deref(), Some("https://risk.example.com/agui"));
        // Installed and switched off, as the assistants are: listed on the
        // Skills page, in no assistant's catalogue until the person switches it on.
        let skills = SkillStore::load(&data);
        assert!(skills.get("find-a-doc").is_some());
        assert!(!skills.is_on("find-a-doc"));
        assert!(!skills.catalog_text().contains("find-a-doc"));

        // Importing again changes nothing: everything is already here, and is kept.
        let again = plan(&pkg, &reg, &SkillStore::load(&data)).unwrap();
        assert!(again.personas.iter().all(|p| !p.problems.is_empty()));
        assert!(again.skills.iter().filter(|s| s.name == "find-a-doc").all(|s| s.problems[0].contains("already")));
        let _ = std::fs::remove_dir_all(&data);
        let _ = std::fs::remove_dir_all(&pkg);
    }

    #[test]
    fn a_folder_that_isnt_a_package_says_so() {
        let empty = temp("empty");
        let e = plan(&empty, &PersonaRegistry::load(&empty), &SkillStore::load(&empty)).unwrap_err();
        assert!(e.contains("isn't an agent package"), "{e}");
        let _ = std::fs::remove_dir_all(&empty);
    }
