//! against a real vault on disk

use super::*;

    //
    // The tests above assert on hand-built `Recall`s, which proves the rendering and
    // proves nothing about the reading. These write four real stores to a temp
    // directory and go through `census`/`recall`, so a store that stops being read —
    // a renamed folder, a changed constructor — fails here rather than quietly
    // dropping a quarter of the brain out of every answer.

    /// Self-contained temp dir, matching `memory::tempdir` rather than adding a dep.
    pub(super) struct Tmp(pub(super) std::path::PathBuf);

    impl Tmp {
        pub(super) fn new() -> Self {
            use std::sync::atomic::{AtomicU64, Ordering};
            static N: AtomicU64 = AtomicU64::new(0);
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let p = std::env::temp_dir()
                .join(format!("brain-test-{}-{}", nanos, N.fetch_add(1, Ordering::SeqCst)));
            std::fs::create_dir_all(&p).unwrap();
            Self(p)
        }
    }

    impl Drop for Tmp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// A vault holding one of everything, and settings pointed at a port nothing is
    /// listening on — so embedding fails fast and both recalls take their documented
    /// keyword fallback. Without that, this test would pass or hang depending on
    /// whether the machine running it happens to have Ollama up.
    pub(super) fn seeded_vault() -> (Tmp, MemoryEngine, Settings) {
        let tmp = Tmp::new();
        let vault = tmp.0.join("vault");
        std::fs::create_dir_all(&vault).unwrap();

        let engine = MemoryEngine::new(vault.clone());
        engine
            .save(&crate::core::memory::MemoryInput {
                title: "Postgres migration plan".into(),
                category: "repo".into(),
                summary: "moving the primary to Postgres 16".into(),
                body: "We agreed to cut over the primary during the October freeze.".into(),
                ..Default::default()
            })
            .unwrap();
        engine
            .save(&crate::core::memory::MemoryInput {
                title: "Lunch order".into(),
                category: "email".into(),
                body: "unrelated to anything".into(),
                ..Default::default()
            })
            .unwrap();

        // Twenty starred repos among a pile of other GitHub activity — the shape that
        // defeated top-k recall on the real vault. None of these mention Postgres, so
        // a Postgres-flavoured query will never retrieve them; only the inventory can.
        for i in 0..20 {
            engine
                .save(&crate::core::memory::MemoryInput {
                    title: format!("Starred owner/repo-{i:02}"),
                    category: "GitHub".into(),
                    body: "GitHub WatchEvent.".into(),
                    ..Default::default()
                })
                .unwrap();
        }

        let journal = Journal::new(&vault);
        journal.write_core("Prefers short answers about Postgres and infrastructure.").unwrap();
        journal.append_daily("2026-08-12", "Chose pgbouncer for Postgres pooling.", "chat").unwrap();

        Knowledge::new(&vault)
            .write("Postgres", "concepts", "Connection pooling notes for Postgres.")
            .unwrap();

        let mut settings = Settings::default();
        settings.ollama_url = "http://127.0.0.1:1".into();
        (tmp, engine, settings)
    }
