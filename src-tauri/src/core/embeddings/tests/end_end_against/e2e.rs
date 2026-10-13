use super::super::*;

    #[tokio::test]
    #[ignore = "requires Ollama with an embed model"]
    pub(in crate::core::embeddings::tests) async fn e2e_a_fact_past_the_old_cap_is_reachable() {
        use crate::core::memory::{MemoryEngine, MemoryInput, Origin};
        // BRAIN-RESEARCH-2026 §5(a) as an end-to-end check, and the second version of
        // it. The first put the buried fact in a memory called "Operations log" and
        // asked about coffee machines — which passed with the fact truncated away,
        // because the *title* was already the best match in the vault. A test that
        // passes under the defect it was written for is worse than no test.
        //
        // So: nothing outside the buried section may help. The title, the summary and
        // four thousand characters of body are all about invoices, and the only text
        // anywhere in the vault that bears on the question sits after all of it.
        let dir = std::env::temp_dir().join(format!("bba-chunk-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir_all(&dir).unwrap();
        let eng = MemoryEngine::new(dir.join("vault"));
        let padding: String = (0..90)
            .map(|i| format!("Invoice {i} was reconciled against the ledger and filed.\n\n"))
            .collect();
        assert!(padding.len() > 4_000, "the fact has to sit past the old 2,000-character cap");
        eng.save(&MemoryInput {
            title: "Invoice archive".into(),
            category: "Finance".into(),
            tags: vec![],
            summary: "reconciled invoices, filed by quarter".into(),
            body: format!("{padding}\n# Kitchen\nThe office espresso machine is descaled every second Tuesday.\n"),
            filename: None,
            origin: Origin::External,
            key: None,
            supersedes: None,
        })
        .unwrap();

        // A distractor that beats the *truncated* memory and loses to the buried
        // section. Without it the assertion is vacuous: a lone memory clears the
        // floor whatever it says, because nothing else is competing.
        eng.save(&MemoryInput {
            title: "Coffee and kitchen supplies".into(),
            category: "Office".into(),
            tags: vec![],
            summary: "ordering beans and the cleaning rota".into(),
            body: "We order beans monthly. The kitchen rota covers washing up and restocking.".into(),
            filename: None,
            origin: Origin::External,
            key: None,
            supersedes: None,
        })
        .unwrap();

        let s = crate::core::embeddings::Settings::default();
        let hits = recall(&eng, &s, &dir, "how often is the espresso machine descaled?", 3).await;
        assert_eq!(
            hits.first().map(|h| h.title.as_str()),
            Some("Invoice archive"),
            "the buried fact lost to a memory that only sounds relevant — this is the \
             one-vector-per-file defect: {hits:?}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    #[ignore = "requires Ollama with an embed model"]
    pub(in crate::core::embeddings::tests) async fn e2e_semantic_recall_surfaces_relevant_memory() {
        use crate::core::memory::{MemoryEngine, MemoryInput, Origin};
        let dir = std::env::temp_dir().join(format!("bba-rag-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir_all(&dir).unwrap();
        let vault = dir.join("vault");
        let eng = MemoryEngine::new(vault);
        eng.save(&MemoryInput { title: "Apollo launch plan".into(), category: "Work".into(), tags: vec![], summary: "rocket launch schedule".into(), body: "We finalized the rocket launch schedule and crew assignments.".into(), filename: None, origin: Origin::External, key: None, supersedes: None }).unwrap();
        eng.save(&MemoryInput { title: "Lunch with Maria".into(), category: "Personal".into(), tags: vec![], summary: "italian food".into(), body: "Had pasta and tiramisu downtown.".into(), filename: None, origin: Origin::External, key: None, supersedes: None }).unwrap();

        let s = crate::core::embeddings::Settings::default();
        let hits = recall(&eng, &s, &dir, "when is the spacecraft taking off?", 2).await;
        assert!(!hits.is_empty(), "semantic recall returned nothing");
        assert_eq!(hits[0].title, "Apollo launch plan", "most relevant memory should rank first");

        let _ = std::fs::remove_dir_all(&dir);
    }
