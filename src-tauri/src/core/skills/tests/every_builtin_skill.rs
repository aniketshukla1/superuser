use super::*;

    #[test]
    pub(super) fn parses_allowed_tools_string_and_list() {
        let s = parse_skill("---\nname: PDF\ndescription: work with pdfs\nallowed-tools: read_file, write_file\n---\ndo pdf things", "pdf", None).unwrap();
        assert_eq!(s.name, "PDF");
        assert_eq!(s.allowed_tools, vec!["read_file", "write_file"]);
        assert!(s.instructions.contains("pdf things"));

        let s2 = parse_skill("---\nname: X\nallowed-tools: [a, b, c]\n---\nbody", "x", None).unwrap();
        assert_eq!(s2.allowed_tools, vec!["a", "b", "c"]);
    }

    #[test]
    pub(super) fn derives_name_from_folder() {
        let s = parse_skill("---\ndescription: d\n---\nbody", "my-skill", None).unwrap();
        assert_eq!(s.name, "my-skill");
    }

    #[test]
    pub(super) fn rejects_no_body() {
        assert!(parse_skill("---\nname: x\n---\n", "x", None).is_err());
    }

    #[test]
    pub(super) fn store_loads_and_catalogs() {
        static SCRATCH_N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let seq = SCRATCH_N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("bba-skills-{}-{}-{}", std::process::id(), n, seq));
        let skill_dir = root.join("skills").join("greeter");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), "---\nname: greeter\ndescription: says hi\n---\nSay hello warmly.").unwrap();

        let store = SkillStore::load(&root);
        // A user skill loads alongside the shipped deliverable templates.
        assert!(store.names().contains(&"greeter".to_string()));
        assert!(store.names().contains(&"financial-budget".to_string()));
        assert!(store.catalog_text().contains("greeter: says hi"));
        assert!(store.get("greeter").unwrap().instructions.contains("hello"));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    pub(super) fn an_assistants_catalogue_is_narrowed_to_what_it_was_given() {
        let store = SkillStore::load(std::path::Path::new("/nonexistent"));
        let all = store.catalog_text();
        assert!(all.contains("semgrep-review") && all.contains("ui-component"));

        let security = store.catalog_text_for(&["semgrep-review".into(), "secret-scan".into()]);
        assert!(security.contains("semgrep-review"));
        assert!(!security.contains("ui-component"), "a list is a list:\n{security}");

        // Empty is not "nothing" — it is the generalist's "everything installed",
        // which is what every assistant did before lists existed.
        assert_eq!(store.catalog_text_for(&[]), all);
    }

    #[test]
    pub(super) fn a_list_naming_an_uninstalled_skill_narrows_rather_than_widens() {
        // Falling back to everything would hand an assistant procedures nobody gave
        // it, which is the opposite of what declaring a list means.
        let store = SkillStore::load(std::path::Path::new("/nonexistent"));
        assert_eq!(store.catalog_text_for(&["no-such-skill".into()]), "");
        assert_eq!(store.missing_from(&["no-such-skill".into(), "secret-scan".into()]), vec!["no-such-skill".to_string()]);
    }

    #[test]
    pub(super) fn every_shipped_assistant_names_skills_that_exist() {
        // A manifest naming a skill nobody ships leaves that assistant with a
        // narrower catalogue than intended, silently.
        let store = SkillStore::load(std::path::Path::new("/nonexistent"));
        for p in crate::core::personas::PersonaRegistry::load(std::path::Path::new("/nonexistent")).list_all() {
            let missing = store.missing_from(&p.skills);
            assert!(missing.is_empty(), "assistant '{}' names skills nothing ships: {missing:?}", p.id);
        }
    }

    /// A skill is a PROCEDURE. These are the properties that distinguish one from a
    /// paragraph of advice, checked because the difference is easy to lose while
    /// writing the twentieth.
    #[test]
    pub(super) fn every_builtin_skill_is_a_procedure_and_not_a_paragraph() {
        let store = SkillStore::load(std::path::Path::new("/nonexistent"));
        let mut descriptions: Vec<String> = Vec::new();
        for (name, _) in BUILTIN_SKILLS {
            let s = store.get(name).unwrap();

            // Structure, in one of the two shapes a skill legitimately takes: a
            // PROCEDURE with ordered steps, or a TEMPLATE that outlines the document
            // it produces. `meeting-brief` is the second and has no steps at all —
            // demanding numbers of it would have been the test dictating the form
            // rather than checking the property. What both have, and a paragraph of
            // advice does not, is structure a reader can follow.
            let steps = s.instructions.lines().filter(|l| l.trim_start().starts_with(|c: char| c.is_ascii_digit())).count();
            let headings = s.instructions.lines().filter(|l| l.trim_start().starts_with('#')).count();
            assert!(
                steps >= 3 || headings >= 3,
                "'{name}' has {steps} steps and {headings} headings — that is advice, not a skill"
            );

            // A description that restates the name tells a picker nothing, and the
            // "/" menu is exactly a picker.
            let d = s.description.to_lowercase();
            assert!(d.len() > 20, "'{name}' has a description too short to choose by: {d:?}");
            assert!(d != name.replace('-', " "), "'{name}' description just restates its name");
            descriptions.push(d);
        }
        // Two skills that describe themselves identically cannot both be picked
        // correctly, whatever the menu shows.
        let unique: std::collections::BTreeSet<&String> = descriptions.iter().collect();
        assert_eq!(unique.len(), descriptions.len(), "two built-in skills share a description");
    }

    #[test]
    pub(super) fn builtin_skills_all_parse_and_produce_a_file() {
        // A skill that doesn't name the artifact it writes is just a chat prompt.
        let store = SkillStore::load(std::path::Path::new("/nonexistent"));
        for (name, _) in BUILTIN_SKILLS {
            let s = store.get(name).unwrap_or_else(|| panic!("built-in skill '{name}' failed to parse"));
            assert!(!s.description.is_empty(), "'{name}' needs a description");
            // The deliverable has to be NAMED — that is what separates a skill from a
            // chat prompt. It is not always a file: the security family grew two
            // skills whose deliverable is a review branch, which is just as concrete
            // and just as checkable, so they name the branch prefix instead.
            // `.test.` counts: a skill whose output is code ships a test beside it,
            // and that file is as named and as checkable as a report. It is not a
            // loophole — a skill that mentions no artifact at all still fails.
            let names_file =
                [".csv", ".md", ".json", ".html", ".test."].iter().any(|e| s.instructions.contains(e));
            let names_branch = ["security/", "deps/", "refactor/", "fix/", "feat/"]
                .iter()
                .any(|p| s.instructions.contains(p));
            // A third kind of deliverable, and the reason it is one: `skill-creator` and
            // `persona-creator` produce a skill or an assistant — a file in the app's own
            // data directory, written when a person presses a card. Concrete and checkable
            // by the same standard as a report; it simply does not land in the workspace.
            let names_proposal = ["save_skill", "save_persona"].iter().any(|t| s.instructions.contains(t));
            assert!(
                names_file || names_branch || names_proposal,
                "'{name}' must name what it produces — a file, the branch it ships, or the skill or assistant it proposes"
            );
        }
    }

    #[test]
    pub(super) fn a_user_skill_overrides_a_builtin_of_the_same_name() {
        let dir = std::env::temp_dir().join(format!("bba-skill-override-{}", std::process::id()));
        let sd = dir.join("skills").join("financial-budget");
        fs::create_dir_all(&sd).unwrap();
        fs::write(sd.join("SKILL.md"), "---\nname: financial-budget\ndescription: mine\n---\nMy own way.").unwrap();
        let store = SkillStore::load(&dir);
        assert_eq!(store.get("financial-budget").unwrap().description, "mine");
        let _ = fs::remove_dir_all(&dir);
    }

    pub(super) fn with_user_skill(tag: &str, name: &str) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!("bba-skill-off-{tag}-{}", crate::core::inbox::short_id()));
        let sd = root.join("skills").join(name);
        fs::create_dir_all(&sd).unwrap();
        fs::write(sd.join("SKILL.md"), format!("---\nname: {name}\ndescription: does {name}\n---\nDo {name}.")).unwrap();
        root
    }
