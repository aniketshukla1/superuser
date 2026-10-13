use super::*;

    #[test]
    pub(super) fn recall_indexes_the_evidence_and_not_the_recap() {
        // Both tiers describe the same turns. Chunking both would double-count them
        // in recall and let an interpretation outrank the words it came from.
        let v = scratch();
        let j = Journal::new(&v);
        j.append_daily("2026-08-12", "user: the deploy target is staging", "sess-7").unwrap();
        j.append_recap("2026-08-12", "Deploy target: staging.", "sess-7").unwrap();
        let texts: Vec<String> = j.chunks().into_iter().map(|c| c.text).collect();
        assert!(texts.iter().any(|t| t.contains("the deploy target is staging")));
        assert!(!texts.iter().any(|t| t.contains("Deploy target: staging.")));
        fs::remove_dir_all(&v).ok();
    }

    #[test]
    pub(super) fn the_journal_is_invisible_to_the_vault_listing() {
        let v = scratch();
        let j = Journal::new(&v);
        j.write_core("- The user prefers concise answers.").unwrap();
        j.append_daily("2026-08-12", "something", "").unwrap();
        j.write_dream("2026-08-12", "merged two entries").unwrap();

        // The vault lists notes non-recursively; the journal lives a level down and
        // must never appear there as a stray "MEMORY" note.
        let engine = crate::core::memory::MemoryEngine::new(v.clone());
        assert!(engine.list().is_empty(), "journal files are not vault notes");
        fs::remove_dir_all(&v).ok();
    }

    #[test]
    pub(super) fn core_memory_is_injected_and_capped() {
        let v = scratch();
        let j = Journal::new(&v);
        j.write_core("- Prefers Rust.\n- Ships on Fridays.").unwrap();
        let block = j.system_block();
        assert!(block.contains("Prefers Rust."));
        assert!(block.contains("LONG-TERM MEMORY"));
        assert!(block.contains("the user is right"), "present facts must beat remembered ones");

        // A core memory that grew out of control cannot swallow the context window.
        j.write_core(&"- a very long remembered fact\n".repeat(2_000)).unwrap();
        assert!(j.system_block().chars().count() < CORE_INJECT_CHAR_CAP + 500);
        fs::remove_dir_all(&v).ok();
    }

    #[test]
    pub(super) fn dailies_and_dreams_are_newest_first() {
        let v = scratch();
        let j = Journal::new(&v);
        for d in ["2026-08-10", "2026-08-12", "2026-08-11"] {
            j.append_daily(d, "note", "").unwrap();
            j.write_dream(d, "dreamt").unwrap();
        }
        assert_eq!(j.dailies().iter().map(|e| e.date.clone()).collect::<Vec<_>>(), ["2026-08-12", "2026-08-11", "2026-08-10"]);
        assert_eq!(j.dreams()[0].date, "2026-08-12");
        assert_eq!(j.recent_dailies(2).len(), 2);
        fs::remove_dir_all(&v).ok();
    }

    #[test]
    pub(super) fn a_second_dream_the_same_day_appends_rather_than_erasing_the_first() {
        let v = scratch();
        let j = Journal::new(&v);
        j.write_dream("2026-08-12", "nightly pass: merged two entries").unwrap();
        j.write_dream("2026-08-12", "manual pass: pulled in last week").unwrap();
        let body = &j.dreams()[0].body;
        assert!(body.contains("nightly pass"));
        assert!(body.contains("manual pass"));
        fs::remove_dir_all(&v).ok();
    }

    #[test]
    pub(super) fn core_bullets_and_daily_sections_chunk_separately() {
        let v = scratch();
        let j = Journal::new(&v);
        j.write_core("- Prefers Rust.\n- Ships on Fridays.").unwrap();
        j.append_daily("2026-08-12", "Decided on Postgres.", "").unwrap();
        j.append_daily("2026-08-12", "Shipped the importer.", "").unwrap();

        let chunks = j.chunks();
        let core: Vec<_> = chunks.iter().filter(|c| c.tier == Tier::Core).collect();
        let daily: Vec<_> = chunks.iter().filter(|c| c.tier == Tier::Daily).collect();

        assert_eq!(core.len(), 2, "one chunk per bullet, not one blob");
        assert_eq!(core[0].text, "Prefers Rust.", "list marker stripped");
        assert!(core[0].date.is_empty());

        assert_eq!(daily.len(), 2, "one chunk per timestamped section");
        assert!(daily.iter().all(|c| c.date == "2026-08-12"));
        assert!(daily.iter().any(|c| c.text.contains("Postgres")));
        assert!(daily.iter().any(|c| c.text.contains("importer")));
        // The `# 2026-08-12` title line is structure, not content.
        assert!(!daily.iter().any(|c| c.text.contains("# 2026-08-12")));

        // Ids are stable and unique so an embedding index can key on them. Both
        // daily appends here land in the same minute, which is exactly the case that
        // collides if ids are keyed on the `## HH:MM` heading.
        let ids: std::collections::HashSet<_> = chunks.iter().map(|c| &c.id).collect();
        assert_eq!(ids.len(), chunks.len(), "chunk ids must be unique");
        assert_eq!(j.chunks().iter().map(|c| c.id.clone()).collect::<Vec<_>>(), chunks.iter().map(|c| c.id.clone()).collect::<Vec<_>>());
        fs::remove_dir_all(&v).ok();
    }

    #[test]
    pub(super) fn daily_memory_halves_every_thirty_days() {
        let today = chrono::NaiveDate::from_ymd_opt(2026, 8, 12).unwrap();
        assert!((decay_factor("2026-08-12", today) - 1.0).abs() < 1e-9, "today counts fully");
        assert!((decay_factor("2026-07-13", today) - 0.5).abs() < 0.02, "30 days → half");
        assert!((decay_factor("2026-06-13", today) - 0.25).abs() < 0.02, "60 days → quarter");
        // A future date (clock skew, a hand-written file) must not score above 1.
        assert_eq!(decay_factor("2027-01-01", today), 1.0);
        // An unparseable date is treated as fresh rather than infinitely old: a bad
        // filename should not silently bury a real memory.
        assert_eq!(decay_factor("not-a-date", today), 1.0);
    }

    #[test]
    pub(super) fn keyword_score_is_a_normalized_term_overlap() {
        assert_eq!(keyword_score("postgres migration", "we chose postgres for the migration"), 1.0);
        assert_eq!(keyword_score("postgres migration", "we chose postgres"), 0.5);
        assert_eq!(keyword_score("postgres", "no match here"), 0.0);
        // Short words are dropped, so a query of only stopwords scores nothing
        // rather than matching everything.
        assert_eq!(keyword_score("is at on", "is at on"), 0.0);
        assert_eq!(keyword_score("", "anything"), 0.0);
    }

    #[test]
    pub(super) fn a_well_formed_distillation_parses_into_core_and_diary() {
        let raw = "<memory>\n- Prefers Rust.\n- Ships on Fridays.\n</memory>\n<diary>\nMerged two entries about scheduling.\n</diary>";
        let (core, diary) = parse_dream(raw).unwrap();
        assert_eq!(core, "- Prefers Rust.\n- Ships on Fridays.");
        assert_eq!(diary, "Merged two entries about scheduling.");
    }

    #[test]
    pub(super) fn chatty_models_that_wrap_the_answer_still_parse() {
        let raw = "Sure! Here's the consolidation:\n\n<memory>\n- Lives in Bangalore.\n</memory>\n<diary>\nAdded a location.\n</diary>\n\nHope that helps!";
        let (core, diary) = parse_dream(raw).unwrap();
        assert_eq!(core, "- Lives in Bangalore.");
        assert_eq!(diary, "Added a location.");
    }

    #[test]
    pub(super) fn a_missing_or_empty_memory_section_is_refused() {
        // Each of these would erase everything the assistant knows about the user if
        // it were written, so parsing must fail rather than return an empty core.
        assert!(parse_dream("I couldn't find anything to consolidate.").is_none());
        assert!(parse_dream("<memory>\n\n</memory><diary>nothing</diary>").is_none());
        assert!(parse_dream("<memory>- unterminated").is_none());
        assert!(parse_dream("").is_none());
    }

    #[test]
    pub(super) fn a_diary_may_be_missing_without_losing_the_memory() {
        let (core, diary) = parse_dream("<memory>\n- One fact.\n</memory>").unwrap();
        assert_eq!(core, "- One fact.");
        assert!(diary.is_empty(), "the caller substitutes a default rather than failing");
    }

    #[tokio::test]
    pub(super) async fn a_dream_with_no_daily_material_is_skipped_before_any_model_call() {
        let v = scratch();
        let j = Journal::new(&v);
        j.write_core("- An existing fact.").unwrap();
        // No daily files at all: skipped without touching the network, and the core
        // memory is left exactly as it was.
        let out = deep_dream(&crate::core::settings::Settings::default(), &j, 3).await;
        assert!(matches!(out, DreamOutcome::Skipped(_)));
        assert_eq!(j.read_core().trim(), "- An existing fact.");
        fs::remove_dir_all(&v).ok();
    }
