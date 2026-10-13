use super::*;

    fn scratch() -> PathBuf {
        use std::sync::atomic::{AtomicU32, Ordering};
        static N: AtomicU32 = AtomicU32::new(0);
        let p = std::env::temp_dir().join(format!("sessdir-{}-{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst)));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn a_conversation_pins_its_folder_once_and_never_moves() {
        // The promise the Files card makes, and the reason the folder had to become
        // recorded rather than derived.
        let d = scratch();
        let store = SessionStore::new(&d);
        let s = Session::with_id("s-1", "assistant", "", "m");
        store.save(&s);

        let elsewhere = scratch();
        let legacy = d.join("sessions").join("s-1");
        let first = store.working_dir("s-1", &elsewhere.to_string_lossy(), legacy.clone());

        // Nothing on disk yet, so it takes the configured base.
        assert!(first.starts_with(&elsewhere), "got {first:?}");

        // Now change the setting. The conversation does not follow it.
        let moved_again = scratch();
        let second = store.working_dir("s-1", &moved_again.to_string_lossy(), legacy);
        assert_eq!(first, second, "an existing conversation must not move");
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn the_session_record_alone_does_not_count_as_work() {
        // The record lives in the working folder, so `exists()` is true for every
        // saved session. Using that as the test meant the setting could never
        // apply to anything — caught by the test above before it shipped.
        let d = scratch();
        let store = SessionStore::new(&d);
        store.save(&Session::with_id("s-5", "assistant", "", "m"));
        let legacy = d.join("sessions").join("s-5");
        assert!(legacy.join("session.json").exists(), "the record is there");
        assert!(!super::has_deliverables(&legacy), "but nothing has been produced");

        let elsewhere = scratch();
        let got = store.working_dir("s-5", &elsewhere.to_string_lossy(), legacy);
        assert!(got.starts_with(&elsewhere), "a fresh conversation follows the setting; got {got:?}");
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn a_conversation_that_already_has_a_folder_keeps_it() {
        // The upgrade case: sessions that existed before the field. Their files are
        // where they are, so the folder on disk wins over the new setting.
        let d = scratch();
        let store = SessionStore::new(&d);
        store.save(&Session::with_id("s-2", "assistant", "", "m"));
        let legacy = d.join("sessions").join("s-2");
        fs::create_dir_all(&legacy).unwrap();
        fs::write(legacy.join("report.md"), "already here").unwrap();

        let elsewhere = scratch();
        let got = store.working_dir("s-2", &elsewhere.to_string_lossy(), legacy.clone());
        assert_eq!(got, legacy, "the files are here, so this is the folder");
        assert!(got.join("report.md").exists());
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn no_setting_means_exactly_the_old_behaviour() {
        let d = scratch();
        let store = SessionStore::new(&d);
        store.save(&Session::with_id("s-3", "assistant", "", "m"));
        let legacy = d.join("sessions").join("s-3");
        assert_eq!(store.working_dir("s-3", "", legacy.clone()), legacy);
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn a_run_with_no_session_record_still_gets_a_folder() {
        // Subagents work under a derived id (`<parent>-sub1`) that was never saved.
        let d = scratch();
        let legacy = d.join("sessions").join("s-4-sub1");
        assert_eq!(SessionStore::new(&d).working_dir("s-4-sub1", "/somewhere", legacy.clone()), legacy);
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn a_tilde_is_a_home_directory_not_a_folder_called_tilde() {
        let expanded = shellexpand_home("~/Superuser");
        assert!(!expanded.starts_with('~'), "got {expanded}");
        assert!(expanded.ends_with("Superuser"));
        assert_eq!(shellexpand_home("/abs/path"), "/abs/path");
    }
