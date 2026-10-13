use super::*;

    pub(super) fn scratch() -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(0);
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let p = std::env::temp_dir().join(format!("bba-journal-{}-{}", nanos, N.fetch_add(1, Ordering::SeqCst)));
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    pub(super) fn a_citation_naming_a_day_in_the_material_survives() {
        let days = vec!["2026-08-12".to_string(), "2026-08-13".to_string()];
        let core = "- Prefers concise answers [2026-08-12]\n- Works at Northwind [core]";
        let (out, stripped) = verify_citations(core, &days);
        assert_eq!(stripped, 0);
        assert_eq!(out, core, "a real citation must be left alone");
    }

    #[test]
    pub(super) fn a_citation_to_a_day_that_was_never_shown_is_removed_and_the_fact_is_kept() {
        // The failure this exists for. Asked to cite, models cite — including days
        // they were not given. A false provenance on a line that goes into every
        // future prompt is worse than no provenance, because it reads as evidence.
        let days = vec!["2026-08-12".to_string()];
        let (out, stripped) = verify_citations("- Runs 1000 VMs [2025-01-04]", &days);
        assert_eq!(stripped, 1);
        assert_eq!(out, "- Runs 1000 VMs", "the fact stays; only the false source goes");
    }

    #[test]
    pub(super) fn an_uncited_bullet_is_left_exactly_as_it_is() {
        let days = vec!["2026-08-12".to_string()];
        for line in ["- No citation here", "", "## A heading"] {
            let (out, stripped) = verify_citations(line, &days);
            assert_eq!(stripped, 0, "{line:?} should not have been touched");
            assert_eq!(out, line);
        }
    }

    #[test]
    pub(super) fn every_bullet_is_checked_not_just_the_first() {
        let days = vec!["2026-08-12".to_string()];
        let core = "- one [2026-08-12]\n- two [1999-01-01]\n- three [2026-08-12]\n- four [nope]";
        let (out, stripped) = verify_citations(core, &days);
        assert_eq!(stripped, 2);
        assert_eq!(out, "- one [2026-08-12]\n- two\n- three [2026-08-12]\n- four");
    }

    #[test]
    pub(super) fn nothing_is_created_until_something_is_written() {
        let v = scratch();
        let j = Journal::new(&v);
        assert!(!j.root.exists(), "constructing a journal must not litter the vault");
        assert_eq!(j.read_core(), "");
        assert!(j.dailies().is_empty());
        assert!(j.dreams().is_empty());
        assert!(j.system_block().is_empty(), "an empty core costs no prompt tokens");
        fs::remove_dir_all(&v).ok();
    }

    #[test]
    pub(super) fn empty_appends_do_not_create_a_file() {
        let v = scratch();
        let j = Journal::new(&v);
        j.append_daily("2026-08-12", "   \n  ", "").unwrap();
        assert!(!j.daily_path("2026-08-12").exists(), "a quiet day leaves no empty file");
        fs::remove_dir_all(&v).ok();
    }

    #[test]
    pub(super) fn daily_appends_accumulate_under_timestamps() {
        let v = scratch();
        let j = Journal::new(&v);
        j.append_daily("2026-08-12", "Decided to use Postgres.", "").unwrap();
        j.append_daily("2026-08-12", "Shipped the importer.", "").unwrap();
        let body = j.read_daily("2026-08-12").unwrap();
        assert!(body.starts_with("# 2026-08-12"));
        assert!(body.contains("Decided to use Postgres."));
        assert!(body.contains("Shipped the importer."));
        assert_eq!(body.matches("## ").count(), 2, "one heading per append");
        fs::remove_dir_all(&v).ok();
    }

    #[test]
    pub(super) fn evidence_and_interpretation_are_kept_in_separate_files() {
        let v = scratch();
        let j = Journal::new(&v);
        j.append_daily("2026-08-12", "user: ship it on Friday\nassistant: noted", "sess-7").unwrap();
        j.append_recap("2026-08-12", "Agreed to ship on Friday.", "sess-7").unwrap();

        let daily = j.read_daily("2026-08-12").unwrap();
        let recap = fs::read_to_string(j.recap_path("2026-08-12")).unwrap();

        // The words as said live in one file, the reading of them in the other, and
        // neither leaks into the other — that separation is the whole point.
        assert!(daily.contains("user: ship it on Friday"));
        assert!(!daily.contains("Agreed to ship on Friday."));
        assert!(recap.contains("Agreed to ship on Friday."));
        assert!(!recap.contains("user: ship it on Friday"));

        assert_eq!(j.dailies().len(), 1);
        assert_eq!(j.recaps().len(), 1);
        fs::remove_dir_all(&v).ok();
    }

    #[test]
    pub(super) fn a_trim_journals_what_was_said_and_files_the_summary_separately() {
        // The regression this closes: the daily file used to receive the *summary*,
        // so the nightly pass distilled an interpretation and the words were kept
        // nowhere. Now the transcript is the daily entry and the summary is a recap.
        let v = scratch();
        let j = Journal::new(&v);
        let dropped = vec![
            serde_json::json!({"role": "user", "content": "the deploy target is staging, not prod"}),
            serde_json::json!({"role": "assistant", "content": "understood, staging it is"}),
            // Tool traffic has no string content, so it never becomes evidence prose.
            serde_json::json!({"role": "tool", "content": serde_json::json!([{"type": "tool_result"}])}),
        ];
        j.record_trim("2026-08-12", &dropped, "Deploy target is staging.", "sess-9");

        let daily = j.read_daily("2026-08-12").unwrap();
        assert!(daily.contains("the deploy target is staging, not prod"), "the user's words: {daily}");
        assert!(daily.contains("understood, staging it is"));
        assert!(!daily.contains("Deploy target is staging."), "the summary must not be the evidence");
        assert!(!daily.contains("tool_result"), "tool traffic is not prose");

        let recap = fs::read_to_string(j.recap_path("2026-08-12")).unwrap();
        assert!(recap.contains("Deploy target is staging."));
        fs::remove_dir_all(&v).ok();
    }

    #[test]
    pub(super) fn a_trim_with_no_prose_writes_nothing_at_all() {
        // A turn of pure tool traffic leaves no evidence, so filing a summary of it
        // would be an assertion with nothing behind it.
        let v = scratch();
        let j = Journal::new(&v);
        let dropped = vec![serde_json::json!({"role": "tool", "content": serde_json::json!([{"type": "tool_result"}])})];
        j.record_trim("2026-08-12", &dropped, "Something happened.", "sess-9");
        assert!(!j.daily_path("2026-08-12").exists());
        assert!(!j.recap_path("2026-08-12").exists(), "no evidence, no recap");
        fs::remove_dir_all(&v).ok();
    }

    #[test]
    pub(super) fn an_entry_names_the_session_it_came_from() {
        // A quotation with no way back to the full conversation is not evidence.
        let v = scratch();
        let j = Journal::new(&v);
        j.append_daily("2026-08-12", "user: hello", "sess-42").unwrap();
        assert!(j.read_daily("2026-08-12").unwrap().contains("· sess-42"));
        fs::remove_dir_all(&v).ok();
    }

    #[tokio::test]
    pub(super) async fn consolidation_ignores_recaps_and_needs_evidence() {
        // Recaps alone must not be enough to distil from: that is exactly the
        // summary-of-summaries path this split exists to close. With no daily file,
        // there is nothing to consolidate, however many recaps are on disk.
        let v = scratch();
        let j = Journal::new(&v);
        j.append_recap("2026-08-12", "Agreed to ship on Friday.", "sess-7").unwrap();
        let s = crate::core::settings::Settings::default();
        assert_eq!(
            deep_dream(&s, &j, 3).await,
            DreamOutcome::Skipped("no daily memory to consolidate")
        );
        fs::remove_dir_all(&v).ok();
    }
