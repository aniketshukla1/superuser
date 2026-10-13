use super::*;

    pub(super) fn engine() -> (MemoryEngine, tempdir::TempLike) {
        let dir = tempdir::TempLike::new();
        (MemoryEngine::new(dir.path.clone()), dir)
    }

    /// A sync rewrites what it already has on every pass. Unchanged, the memory keeps the date
    /// it arrived on — the Chief of Staff's "since yesterday" depends on it — and changed, it is
    /// dated now.
    #[test]
    pub(super) fn a_rewrite_that_changes_nothing_keeps_its_date() {
        let (eng, _g) = engine();
        let email = |body: &str| MemoryInput {
            filename: Some("email-7.md".into()),
            title: "Email: Board deck".into(),
            category: "Email".into(),
            summary: "from the CFO".into(),
            body: body.into(),
            origin: Origin::External,
            ..Default::default()
        };
        let date_of = |eng: &MemoryEngine| eng.list().into_iter().find(|m| m.filename == "email-7.md").unwrap().date;
        eng.save(&email("Due Friday.")).unwrap();
        let first = date_of(&eng);
        std::thread::sleep(std::time::Duration::from_millis(15));
        eng.save(&email("Due Friday.")).unwrap();
        assert_eq!(date_of(&eng), first, "the same email synced again is not new");
        std::thread::sleep(std::time::Duration::from_millis(15));
        eng.save(&email("Due Friday. Now due Thursday.")).unwrap();
        assert_ne!(date_of(&eng), first, "changed content is new activity");
    }

    #[test]
    pub(super) fn save_list_recall_roundtrip() {
        let (eng, _g) = engine();
        eng.save(&MemoryInput {
            filename: None,
            origin: Origin::Internal,
            key: None,
            supersedes: None,
            title: "Project Apollo kickoff".into(),
            category: "Work".into(),
            tags: vec!["apollo".into(), "kickoff".into()],
            summary: "Kicked off the Apollo project".into(),
            body: "We discussed the Apollo roadmap and milestones.".into(),
        })
        .unwrap();

        let all = eng.list();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].title, "Project Apollo kickoff");
        assert!(all[0].weight > 0.9, "fresh memory should be hot");

        let cites = eng.recall("apollo roadmap", 5);
        assert_eq!(cites.len(), 1);
        assert!(cites[0].score > 0);
    }

    #[test]
    pub(super) fn recall_ignores_short_and_empty() {
        let (eng, _g) = engine();
        assert!(eng.recall("", 5).is_empty());
        assert!(eng.recall("a to is", 5).is_empty());
    }

    #[test]
    pub(super) fn a_keyed_write_updates_in_place_instead_of_adding_a_second_memory() {
        let (eng, _g) = engine();
        let mk = |body: &str| MemoryInput {
            filename: None,
            key: Some("github:issue:1421".into()),
            supersedes: None,
            title: "Flaky integration test".into(),
            category: "GitHub".into(),
            tags: vec![],
            summary: "an open issue".into(),
            body: body.into(),
            origin: Origin::External,
        };
        let first = eng.save(&mk("still failing on CI")).unwrap();
        let second = eng.save(&mk("fixed by pinning the runner")).unwrap();
        assert_eq!(first, second, "the same upstream item must not become two files");
        assert_eq!(eng.list().len(), 1, "a re-sync left a duplicate: {:?}", eng.list());
        assert!(eng.list()[0].body.contains("pinning the runner"), "the update must win");
    }

    #[test]
    pub(super) fn a_keyed_update_keeps_what_using_it_earned() {
        let (eng, _g) = engine();
        let mk = || MemoryInput {
            filename: None,
            key: Some("notion:page:9".into()),
            supersedes: None,
            title: "Runbook".into(),
            category: "Notion".into(),
            tags: vec![],
            summary: "".into(),
            body: "body".into(),
            origin: Origin::External,
        };
        let f = eng.save(&mk()).unwrap();
        eng.reinforce(&f).unwrap();
        eng.reinforce(&f).unwrap();
        assert_eq!(eng.list()[0].reinforce_count, 2);
        eng.save(&mk()).unwrap();
        // Reinforcement records the user's attention, not the bytes. A connector
        // re-syncing the same page must not reset it.
        assert_eq!(eng.list()[0].reinforce_count, 2, "a re-sync wiped the reinforce count");
    }

    #[test]
    pub(super) fn an_unkeyed_write_still_makes_a_new_file() {
        // The key is opt-in. Two notes the user typed are two notes, even if they
        // happen to be about the same thing.
        let (eng, _g) = engine();
        let mk = || MemoryInput {
            filename: None,
            key: None,
            supersedes: None,
            title: "Thoughts".into(),
            category: "Notes".into(),
            tags: vec![],
            summary: "".into(),
            body: "a thought".into(),
            origin: Origin::Internal,
        };
        eng.save(&mk()).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2));
        eng.save(&mk()).unwrap();
        assert_eq!(eng.list().len(), 2);
    }

    #[test]
    pub(super) fn a_superseded_memory_drops_out_of_recall_and_stays_on_disk() {
        let (eng, _g) = engine();
        let old = eng
            .save(&MemoryInput {
                filename: Some("old.md".into()),
                key: None,
                supersedes: None,
                title: "Storage vendor".into(),
                category: "Ops".into(),
                tags: vec![],
                summary: "".into(),
                body: "The storage vendor is Northwind.".into(),
                origin: Origin::External,
            })
            .unwrap();
        eng.save(&MemoryInput {
            filename: Some("new.md".into()),
            key: None,
            supersedes: Some(old.clone()),
            title: "Storage vendor".into(),
            category: "Ops".into(),
            tags: vec![],
            summary: "".into(),
            body: "The storage vendor is Calder.".into(),
            origin: Origin::External,
        })
        .unwrap();

        let hits = eng.recall_scored("storage vendor", 10);
        assert_eq!(hits.len(), 1, "the superseded memory is still being recalled: {hits:?}");
        assert!(hits[0].0.body.contains("Calder"));
        // Declared, not deleted. It is the user's file and it stays browsable.
        assert!(eng.vault_path.join("old.md").exists(), "supersession must not delete anything");
        assert_eq!(eng.list().len(), 2, "the UI still lists both");
    }

    #[test]
    pub(super) fn freshness_marks_the_older_of_two_entries_sharing_a_title() {
        let older = Memory {
            id: "a".into(), filename: "a.md".into(), key: String::new(), supersedes: String::new(), suspect: false,
            title: "Platform on-call".into(), category: "Eng".into(), tags: vec![], summary: String::new(),
            body: String::new(), date: "2026-01-05T00:00:00+00:00".into(), weight: 1.0,
            origin: Origin::External, reinforce_count: 0, last_interaction: String::new(),
        };
        let newer = Memory { id: "b".into(), filename: "b.md".into(), date: "2026-08-20T00:00:00+00:00".into(), ..older.clone() };
        let other = Memory { id: "c".into(), filename: "c.md".into(), title: "Something else".into(), ..older.clone() };

        let f = MemoryEngine::freshness(&[older.clone(), newer.clone(), other.clone()]);
        assert_eq!(f["a.md"].as_deref(), Some("2026-08-20T00:00:00+00:00"), "the older one is overtaken");
        assert_eq!(f["b.md"], None, "the newest of a title is not overtaken");
        assert_eq!(f["c.md"], None, "a title with one entry is never overtaken");
    }
