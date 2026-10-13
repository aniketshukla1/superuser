use super::super::*;

    #[tokio::test]
    #[ignore = "requires Ollama with an embed model (e.g. nomic-embed-text)"]
    pub(in crate::core::embeddings::tests) async fn e2e_embeddings_rank_related_higher() {
        let s = crate::core::embeddings::Settings::default();
        let vecs = embed(&s, vec![
            "the cat sat on the mat".into(),
            "a kitten is resting on a rug".into(),
            "quarterly revenue and tax filings".into(),
        ]).await.expect("embed should succeed");
        assert_eq!(vecs.len(), 3);
        assert!(!vecs[0].is_empty(), "embedding should have dimensions");
        let related = cosine(&vecs[0], &vecs[1]);
        let unrelated = cosine(&vecs[0], &vecs[2]);
        assert!(related > unrelated, "related ({related}) should beat unrelated ({unrelated})");
    }

    /// The half the unit tests can't reach: matching on *meaning* with no shared
    /// terms. "which database did we pick" has no word in common with "Postgres over
    /// MySQL", so only the vector side can connect them.
    #[tokio::test]
    #[ignore = "requires Ollama with an embed model (e.g. nomic-embed-text)"]
    pub(in crate::core::embeddings::tests) async fn e2e_journal_recall_matches_on_meaning_not_words() {
        let dir = scratch();
        let j = crate::core::journal::Journal::new(&dir);
        j.write_core("- Prefers Postgres over MySQL for new services.").unwrap();
        j.append_daily(&crate::core::journal::today(), "Spent the afternoon on a flaky reconnect in the websocket layer.", "").unwrap();

        let s = Settings::default();
        let hits = recall_journal(&j, &s, &dir, "which database did we settle on?", 5).await;
        assert!(!hits.is_empty(), "semantic recall returned nothing");
        assert!(hits[0].text.contains("Postgres"), "got: {:?}", hits[0].text);

        let hits = recall_journal(&j, &s, &dir, "the socket kept dropping", 5).await;
        assert!(hits.iter().any(|h| h.text.contains("reconnect")), "got: {hits:?}");

        // The index is written once and reused: a second identical call must not
        // re-embed (it is keyed by chunk id + content hash).
        let before = std::fs::metadata(journal_index_path(&dir)).unwrap().modified().unwrap();
        let _ = recall_journal(&j, &s, &dir, "which database did we settle on?", 5).await;
        assert_eq!(std::fs::metadata(journal_index_path(&dir)).unwrap().modified().unwrap(), before, "unchanged chunks must not re-embed");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    pub(in crate::core::embeddings::tests) fn a_memory_splits_on_its_headings() {
        use crate::core::memory::{Memory, Origin};
        let m = Memory {
            id: "notes".into(),
            key: String::new(),
            supersedes: String::new(),
            suspect: false,
            filename: "notes.md".into(),
            title: "Runbook".into(),
            category: "Eng".into(),
            tags: vec![],
            date: "2026-01-01".into(),
            summary: "how to run it".into(),
            body: "intro line\n\n# Rollback\nrun the rollback script\n\n# Escalation\npage the on-call\n".into(),
            weight: 1.0,
            reinforce_count: 0,
            last_interaction: "2026-01-01".into(),
            origin: Origin::Internal,
        };
        let chunks = chunks_of(&m);
        assert_eq!(chunks.len(), 3, "intro plus two headings: {chunks:?}");
        assert_eq!(chunks[0].0, "notes.md#0");
        assert_eq!(chunks[2].0, "notes.md#2");
        // Every chunk carries the title, or a chunk retrieved on its own says nothing
        // about what it belongs to.
        for (_, text) in &chunks {
            assert!(text.contains("Runbook"), "chunk lost its title: {text:?}");
        }
        assert!(chunks[1].1.contains("rollback script"));
        assert!(chunks[2].1.contains("page the on-call"));
    }

    #[test]
    pub(in crate::core::embeddings::tests) fn a_long_memory_is_not_one_vector() {
        use crate::core::memory::{Memory, Origin};
        // The defect, in a test. A memory with no headings and 8,000 characters used
        // to be embedded on its first 2,000 and the rest was unreachable forever.
        let body = (0..200).map(|i| format!("line {i} of the log\n\n")).collect::<String>();
        assert!(body.len() > 3_000);
        let m = Memory {
            id: "long".into(),
            key: String::new(),
            supersedes: String::new(),
            suspect: false,
            filename: "long.md".into(),
            title: "Long".into(),
            category: "Eng".into(),
            tags: vec![],
            date: "2026-01-01".into(),
            summary: "".into(),
            body,
            weight: 1.0,
            reinforce_count: 0,
            last_interaction: "2026-01-01".into(),
            origin: Origin::Internal,
        };
        let chunks = chunks_of(&m);
        assert!(chunks.len() > 1, "a heading-less wall of text still has to be divided");
        // And the end of it is reachable, which is the whole point.
        let last = &chunks.last().unwrap().1;
        assert!(last.contains("line 199"), "the tail of the memory must be indexed: {last:?}");
    }

    #[test]
    pub(in crate::core::embeddings::tests) fn an_empty_memory_still_indexes_on_its_title() {
        use crate::core::memory::{Memory, Origin};
        let m = Memory {
            id: "stub".into(),
            key: String::new(),
            supersedes: String::new(),
            suspect: false,
            filename: "stub.md".into(),
            title: "A title and nothing else".into(),
            category: "Eng".into(),
            tags: vec![],
            date: "2026-01-01".into(),
            summary: "still findable".into(),
            body: "".into(),
            weight: 1.0,
            reinforce_count: 0,
            last_interaction: "2026-01-01".into(),
            origin: Origin::Internal,
        };
        let chunks = chunks_of(&m);
        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].1.contains("A title and nothing else"));
    }
