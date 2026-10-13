use super::*;

    fn scratch() -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(0);
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let p = std::env::temp_dir().join(format!("bba-migrate-{}-{}", nanos, N.fetch_add(1, Ordering::SeqCst)));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn install_at(dir: &std::path::Path, marker: &str) {
        std::fs::create_dir_all(dir.join("vault")).unwrap();
        std::fs::write(dir.join("settings.json"), format!("{{\"marker\":\"{marker}\"}}")).unwrap();
        std::fs::write(dir.join("vault/note.md"), marker).unwrap();
    }

    #[test]
    fn an_install_under_the_old_name_is_carried_over_and_left_intact() {
        let base = scratch();
        let (old, new) = (base.join("ai.bigbrain.app"), base.join("ai.superuser.app"));
        install_at(&old, "theirs");

        migrate_legacy_data(&old, &new);
        assert_eq!(std::fs::read_to_string(new.join("vault/note.md")).unwrap(), "theirs");
        // Copied, not moved: if anything about this is wrong, their data is still
        // exactly where it was.
        assert!(old.join("settings.json").exists());
        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn a_live_install_is_never_overwritten() {
        let base = scratch();
        let (old, new) = (base.join("ai.bigbrain.app"), base.join("ai.superuser.app"));
        install_at(&old, "old");
        install_at(&new, "current");

        // The guard lives in resolve_data_dir, but assert the invariant that matters:
        // a directory holding an install must not be a migration target.
        assert!(new.join("settings.json").exists());
        assert_eq!(std::fs::read_to_string(new.join("vault/note.md")).unwrap(), "current");
        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn an_empty_directory_does_not_count_as_an_install() {
        let base = scratch();
        let (old, new) = (base.join("ai.bigbrain.app"), base.join("ai.superuser.app"));
        install_at(&old, "theirs");
        // Several code paths create the data dir eagerly just to write into it, so
        // this is the real situation on an upgrading machine — and treating it as
        // an install would hide everything they had.
        std::fs::create_dir_all(&new).unwrap();
        assert!(!new.join("settings.json").exists() && !new.join("vault").exists());

        migrate_legacy_data(&old, &new);
        assert_eq!(std::fs::read_to_string(new.join("vault/note.md")).unwrap(), "theirs");
        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn a_fresh_machine_migrates_nothing() {
        let base = scratch();
        let (old, new) = (base.join("ai.bigbrain.app"), base.join("ai.superuser.app"));
        migrate_legacy_data(&old, &new);
        assert!(!new.exists(), "no old install means nothing to create");

        // An old directory that was created but never used is not an install either.
        std::fs::create_dir_all(&old).unwrap();
        migrate_legacy_data(&old, &new);
        assert!(!new.exists());
        std::fs::remove_dir_all(&base).ok();
    }
