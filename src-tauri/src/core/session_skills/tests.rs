use super::*;
    use crate::core::skills::SkillStore;

    fn scratch() -> PathBuf {
        use std::sync::atomic::{AtomicU32, Ordering};
        static N: AtomicU32 = AtomicU32::new(0);
        let p = std::env::temp_dir().join(format!("sesskills-{}-{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst)));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn the_menu_lists_every_installed_skill_and_defaults_to_available() {
        let d = scratch();
        let store = SkillStore::load(&d);
        let rows = SessionSkills::load(&d).menu("s1", &store, &[]);
        assert!(!rows.is_empty(), "the built-ins ship, so a fresh session has a menu");
        assert!(rows.iter().all(|r| r.enabled), "unset means available");
        assert!(rows.iter().any(|r| r.name == "semgrep-review"), "including the security ones");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn the_menu_shows_only_what_the_assistant_was_given() {
        // Otherwise a person could force-run a skill the assistant never had — the
        // narrowing undone by the surface that displays it.
        let d = scratch();
        let store = SkillStore::load(&d);
        let rows = SessionSkills::load(&d).menu("s1", &store, &["secret-scan".to_string()]);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "secret-scan");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_mute_is_per_session_not_per_skill() {
        // Turning one off for this conversation must not turn it off everywhere —
        // that is what the Skills settings page is for.
        let d = scratch();
        let store = SkillStore::load(&d);
        let s = SessionSkills::load(&d);
        s.set("s1", "secret-scan", false).unwrap();

        let muted = s.menu("s1", &store, &[]);
        assert!(!muted.iter().find(|r| r.name == "secret-scan").unwrap().enabled);
        assert!(!s.is_enabled("s1", "secret-scan"));

        let other = s.menu("s2", &store, &[]);
        assert!(other.iter().find(|r| r.name == "secret-scan").unwrap().enabled, "another session is untouched");
        assert!(s.is_enabled("s2", "secret-scan"));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn clearing_returns_a_skill_to_available_and_tidies_up() {
        let d = scratch();
        let s = SessionSkills::load(&d);
        s.set("s1", "secret-scan", false).unwrap();
        s.clear("s1", "secret-scan").unwrap();
        assert!(s.is_enabled("s1", "secret-scan"));
        // The session's entry goes with its last mute; the store records departures
        // from the default, so an empty map is not a state worth keeping.
        let raw = std::fs::read_to_string(d.join("session-skills.json")).unwrap();
        assert!(!raw.contains("s1"), "{raw}");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_choice_survives_a_reload_and_a_broken_file_does_not_break_the_menu() {
        let d = scratch();
        SessionSkills::load(&d).set("s1", "secret-scan", false).unwrap();
        assert!(!SessionSkills::load(&d).is_enabled("s1", "secret-scan"));

        std::fs::write(d.join("session-skills.json"), "{ not json").unwrap();
        assert!(SessionSkills::load(&d).is_enabled("s1", "secret-scan"), "a corrupt file means no mutes, not no skills");
        let _ = std::fs::remove_dir_all(&d);
    }
