use super::*;

    #[test]
    pub(super) fn cosine_basics() {
        assert!((cosine(&[1.0, 0.0], &[1.0, 0.0]) - 1.0).abs() < 1e-6);
        assert!(cosine(&[1.0, 0.0], &[0.0, 1.0]).abs() < 1e-6);
        assert_eq!(cosine(&[], &[]), 0.0);
        assert_eq!(cosine(&[1.0], &[1.0, 2.0]), 0.0);
    }

    pub(super) fn scratch() -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(0);
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let p = std::env::temp_dir().join(format!("bba-jrecall-{}-{}", nanos, N.fetch_add(1, Ordering::SeqCst)));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    /// With no embedding model reachable (the case in unit tests), recall falls back
    /// to pure keyword scoring — the point being that it still works rather than
    /// returning nothing.
    #[tokio::test]
    pub(super) async fn journal_recall_works_without_an_embedding_model() {
        let dir = scratch();
        let j = crate::core::journal::Journal::new(&dir);
        j.write_core("- Prefers Postgres over MySQL for new services.").unwrap();
        j.append_daily(&crate::core::journal::today(), "Debugged a flaky websocket reconnect in the GUI.", "").unwrap();

        // Settings point at a local Ollama that isn't running in CI; embedding fails
        // and the blend degrades to keyword-only. So these queries share terms with
        // the text — matching on *meaning* is the vector half's job and is covered by
        // the `--ignored` end-to-end tests below.
        let s = Settings { ollama_url: "http://127.0.0.1:1".into(), ..Settings::default() };
        let hits = recall_journal(&j, &s, &dir, "postgres for new services", 5).await;
        assert!(!hits.is_empty(), "keyword fallback must still return matches");
        assert!(hits[0].text.contains("Postgres"));

        let hits = recall_journal(&j, &s, &dir, "websocket reconnect", 5).await;
        assert!(hits.iter().any(|h| h.text.contains("websocket")));

        // Nothing related → nothing returned, rather than the least-bad chunk.
        assert!(recall_journal(&j, &s, &dir, "tarragon risotto", 5).await.is_empty());
        assert!(recall_journal(&j, &s, &dir, "   ", 5).await.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    pub(super) async fn old_daily_memory_ranks_below_an_equally_good_recent_one() {
        let dir = scratch();
        let j = crate::core::journal::Journal::new(&dir);
        let today = chrono::Local::now().date_naive();
        let older = (today - chrono::Duration::days(30)).format("%Y-%m-%d").to_string();
        let recent = today.format("%Y-%m-%d").to_string();
        // Identical text, different dates — so only decay can separate them.
        j.append_daily(&older, "The deployment target is the staging cluster.", "").unwrap();
        j.append_daily(&recent, "The deployment target is the staging cluster.", "").unwrap();

        let s = Settings { ollama_url: "http://127.0.0.1:1".into(), ..Settings::default() };
        let hits = recall_journal(&j, &s, &dir, "deployment target cluster", 5).await;
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].date, recent, "recent wins on identical content");
        assert!((hits[1].score / hits[0].score - 0.5).abs() < 0.05, "one half-life older ≈ half the score");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    pub(super) async fn memory_old_enough_to_be_noise_drops_out_entirely() {
        let dir = scratch();
        let j = crate::core::journal::Journal::new(&dir);
        let ancient = (chrono::Local::now().date_naive() - chrono::Duration::days(90)).format("%Y-%m-%d").to_string();
        j.append_daily(&ancient, "The deployment target is the staging cluster.", "").unwrap();

        let s = Settings { ollama_url: "http://127.0.0.1:1".into(), ..Settings::default() };
        // Three half-lives → 12.5% of a perfect match, under the relevance floor. A
        // conversation summary from three months ago is not worth prompt tokens
        // unless it's a much stronger match than this.
        assert!(recall_journal(&j, &s, &dir, "deployment target cluster", 5).await.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    pub(super) async fn core_memory_does_not_fade() {
        let dir = scratch();
        let j = crate::core::journal::Journal::new(&dir);
        j.write_core("- The deployment target is the staging cluster.").unwrap();
        let old = (chrono::Local::now().date_naive() - chrono::Duration::days(120)).format("%Y-%m-%d").to_string();
        j.append_daily(&old, "The deployment target is the staging cluster.", "").unwrap();

        let s = Settings { ollama_url: "http://127.0.0.1:1".into(), ..Settings::default() };
        let hits = recall_journal(&j, &s, &dir, "deployment target cluster", 5).await;
        assert_eq!(hits[0].tier, crate::core::journal::Tier::Core, "distilled memory keeps its full weight");
        assert!((hits[0].score - 1.0).abs() < 1e-9, "no decay applied to core");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    pub(super) fn the_journal_block_is_empty_when_nothing_matched() {
        use crate::core::journal::Tier;
        assert_eq!(journal_block(&[]), "", "no heading over an empty list");
        let hits = vec![
            JournalHit { id: "core#0".into(), tier: Tier::Core, date: String::new(), text: "Prefers Rust.".into(), score: 0.9 },
            JournalHit { id: "daily/2026-08-12#14:20".into(), tier: Tier::Daily, date: "2026-08-12".into(), text: "Chose Postgres.".into(), score: 0.5 },
        ];
        let b = journal_block(&hits);
        assert!(b.contains("- Prefers Rust."));
        assert!(b.contains("- (2026-08-12) Chose Postgres."), "dated so the model can weigh recency");
    }

    #[test]
    pub(super) fn hash_changes_with_content() {
        assert_ne!(content_hash("a", "b"), content_hash("a", "c"));
        assert_eq!(content_hash("a", "b"), content_hash("a", "b"));
    }
