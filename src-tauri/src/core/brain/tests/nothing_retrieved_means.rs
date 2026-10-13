use super::*;

    #[test]
    pub(super) fn mention_is_parsed_only_at_the_start_of_a_message() {
        assert_eq!(parse_mention("@brain what did I decide?"), Some("what did I decide?".into()));
        assert_eq!(parse_mention("  @Brain  spaced out  "), Some("spaced out".into()));
        assert_eq!(parse_mention("@BRAIN: with a colon"), Some("with a colon".into()));
        // A bare mention is a request for help, not an empty question.
        assert_eq!(parse_mention("@brain"), Some(String::new()));
        assert_eq!(parse_mention("@brain   "), Some(String::new()));
        // Not a mention.
        assert_eq!(parse_mention("I asked @brain earlier"), None);
        assert_eq!(parse_mention("@brainstorm ideas"), None);
        assert_eq!(parse_mention("what does the brain know?"), None);
        assert_eq!(parse_mention(""), None);
    }

    #[test]
    pub(super) fn clip_cuts_on_character_boundaries() {
        assert_eq!(clip("short", 20), "short");
        assert_eq!(clip("  trimmed  ", 20), "trimmed");
        assert_eq!(clip("abcdef", 3), "abc…");
        // Would panic under byte slicing.
        assert_eq!(clip("héllo wörld", 4), "héll…");
        assert_eq!(clip("日本語テキスト", 3), "日本語…");
    }

    pub(super) fn census_fixture() -> Census {
        Census {
            memories: 42,
            categories: vec![("email".into(), 30), ("repo".into(), 12)],
            oldest: "2026-01-04".into(),
            newest: "2026-09-01".into(),
            journal_days: 9,
            has_core_memory: true,
            knowledge_pages: 3,
            profile_facets: 5,
        }
    }

    #[test]
    pub(super) fn census_block_carries_exact_counts() {
        let b = census_fixture().block();
        assert!(b.contains("Memories: 42"));
        assert!(b.contains("email: 30"));
        assert!(b.contains("repo: 12"));
        assert!(b.contains("2026-01-04 to 2026-09-01"));
        assert!(b.contains("9 daily entries, plus distilled core memory"));
        assert!(b.contains("Knowledge pages: 3"));
    }

    #[test]
    pub(super) fn an_empty_brain_says_so_rather_than_reporting_four_zeroes() {
        let b = Census::default().block();
        assert!(b.contains("empty"));
        assert!(!b.contains("Memories: 0"));
    }

    pub(super) fn memory(title: &str, category: &str, body: &str) -> Memory {
        Memory {
            id: title.into(),
            filename: format!("{title}.md"),
            key: String::new(),
            supersedes: String::new(),
            suspect: false,
            title: title.into(),
            category: category.into(),
            tags: vec![],
            summary: String::new(),
            body: body.into(),
            date: "2026-08-01".into(),
            weight: 1.0,
            origin: Default::default(),
            reinforce_count: 0,
            last_interaction: String::new(),
        }
    }

    pub(super) fn page(title: &str) -> Page {
        Page {
            slug: title.to_lowercase(),
            title: title.into(),
            category: "concepts".into(),
            body: "page body".into(),
            updated_at: "2026-08-02".into(),
            links: vec![],
        }
    }

    pub(super) fn hit(tier: Tier, date: &str, text: &str) -> JournalHit {
        JournalHit { id: date.into(), tier, date: date.into(), text: text.into(), score: 1.0 }
    }

    pub(super) fn full_recall() -> Recall {
        Recall {
            memories: vec![memory("Postgres migration", "repo", "we moved to 16")],
            journal: vec![
                hit(Tier::Core, "", "prefers short answers"),
                hit(Tier::Daily, "2026-08-12", "decided on pgbouncer"),
                hit(Tier::Daily, "2026-08-12", "same day, second entry"),
            ],
            pages: vec![page("Postgres")],
            profile: "\n\n[ABOUT THE USER]\nStyle:\n- concise".into(),
            census: census_fixture(),
            degraded: None,
            inventories: vec![],
            budget: Budget::CLOUD,
        }
    }

    #[test]
    pub(super) fn all_four_stores_reach_the_prompt() {
        let p = system_prompt(&full_recall());
        assert!(p.contains("Postgres migration"), "memories missing");
        assert!(p.contains("decided on pgbouncer"), "journal missing");
        assert!(p.contains("page body"), "knowledge missing");
        assert!(p.contains("ABOUT THE USER"), "profile missing");
        assert!(p.contains("Memories: 42"), "census missing");
    }

    #[test]
    pub(super) fn sources_are_built_from_what_was_retrieved_and_dates_are_deduplicated() {
        let s = full_recall().sources_block();
        assert!(s.contains("Postgres migration (repo)"));
        assert!(s.contains("**Knowledge** — Postgres"));
        assert!(s.contains("core memory"));
        // One entry for the day, not one per chunk.
        assert_eq!(s.matches("2026-08-12").count(), 1);
    }

    #[test]
    pub(super) fn nothing_retrieved_means_no_sources_heading() {
        let r = Recall {
            memories: vec![],
            journal: vec![],
            pages: vec![],
            profile: String::new(),
            census: Census::default(),
            degraded: None,
            inventories: vec![],
            budget: Budget::LOCAL,
        };
        assert!(r.is_empty());
        assert_eq!(r.sources_block(), "");
        // ...and the prompt says the vault was searched and came back empty, so the
        // model reports a gap instead of filling one.
        let p = system_prompt(&r);
        assert!(p.contains("Nothing in the vault matched"));
    }

    #[test]
    pub(super) fn the_local_budget_is_smaller_everywhere_but_still_covers_every_store() {
        let (l, c) = (Budget::LOCAL, Budget::CLOUD);
        assert!(l.memories < c.memories && l.memory_chars < c.memory_chars);
        assert!(l.journal < c.journal && l.pages < c.pages && l.page_chars < c.page_chars);
        assert!(l.memories > 0 && l.journal > 0 && l.pages > 0, "a store cut to zero is a store that stopped existing");
        assert_eq!(Budget::resolve(true), l);
        assert_eq!(Budget::resolve(false), c);
    }

    #[test]
    pub(super) fn excerpts_honour_the_budget() {
        let long = "x".repeat(5000);
        let r = Recall {
            memories: vec![memory("Long", "notes", &long)],
            journal: vec![],
            pages: vec![],
            profile: String::new(),
            census: Census::default(),
            degraded: None,
            inventories: vec![],
            budget: Budget::LOCAL,
        };
        let block = r.block();
        assert!(block.contains('…'));
        assert!(block.matches('x').count() <= Budget::LOCAL.memory_chars);
    }
