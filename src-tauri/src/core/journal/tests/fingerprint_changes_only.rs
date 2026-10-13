use super::*;

    #[tokio::test]
    pub(super) async fn an_unchanged_journal_is_skipped_on_the_second_pass() {
        let v = scratch();
        let j = Journal::new(&v);
        j.append_daily(&today(), "something happened", "").unwrap();
        j.record_fingerprint(j.input_fingerprint(3));
        let out = deep_dream(&crate::core::settings::Settings::default(), &j, 3).await;
        assert_eq!(out, DreamOutcome::Skipped("nothing has changed since the last pass"));
        fs::remove_dir_all(&v).ok();
    }

    #[test]
    pub(super) fn the_fingerprint_changes_only_when_the_material_does() {
        let v = scratch();
        let j = Journal::new(&v);
        j.write_core("- one").unwrap();
        j.append_daily("2026-08-12", "a thing happened", "").unwrap();
        let fp = j.input_fingerprint(3);
        assert_eq!(fp, j.input_fingerprint(3), "stable for unchanged input");

        j.record_fingerprint(fp);
        assert_eq!(j.last_fingerprint(), Some(fp));

        j.append_daily("2026-08-12", "another thing", "").unwrap();
        assert_ne!(j.input_fingerprint(3), fp, "new material must trigger a new pass");
        fs::remove_dir_all(&v).ok();
    }
