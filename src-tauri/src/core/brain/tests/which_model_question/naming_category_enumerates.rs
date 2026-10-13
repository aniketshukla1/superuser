use super::super::*;

    /// The regression, end to end on disk: a question naming a category enumerates
    /// that category exactly, even though not one of its twenty members would have
    /// been retrieved on relevance.
    #[tokio::test]
    pub(in crate::core::brain::tests) async fn naming_a_category_enumerates_it_even_when_recall_would_miss_every_member() {
        let (tmp, engine, settings) = seeded_vault();
        let vault = tmp.0.join("vault");
        let q = "which GitHub repositories have I starred?";
        let r = recall(&engine, &settings, &vault, &tmp.0, q, Budget::LOCAL).await;

        let inv = r.inventories.iter().find(|i| i.category == "GitHub").expect("GitHub not listed");
        assert_eq!(inv.total, 20);
        assert!(inv.complete(), "20 titles fit well inside the cap");

        // Every one of them is in the prompt, whatever recall happened to rank.
        let p = system_prompt(&r);
        for i in 0..20 {
            assert!(p.contains(&format!("Starred owner/repo-{i:02}")), "repo-{i:02} missing");
        }
        assert!(r.sources_block().contains("all 20 memories filed under GitHub"));

        // And a question that names no category still pays nothing for the feature.
        let other = recall(&engine, &settings, &vault, &tmp.0, "the Postgres cutover", Budget::LOCAL).await;
        assert!(other.inventories.is_empty());
        // The word appears in the standing instructions; it is the BLOCK that
        // must be absent, and its header is what marks one.
        assert!(other.inventory_block().is_empty());
        assert!(!system_prompt(&other).contains("[INVENTORY"));
    }

    #[tokio::test]
    pub(in crate::core::brain::tests) async fn an_empty_vault_recalls_nothing_rather_than_failing() {
        let tmp = Tmp::new();
        let vault = tmp.0.join("vault");
        std::fs::create_dir_all(&vault).unwrap();
        let engine = MemoryEngine::new(vault.clone());
        let mut settings = Settings::default();
        settings.ollama_url = "http://127.0.0.1:1".into();

        let r = recall(&engine, &settings, &vault, &tmp.0, "anything", Budget::LOCAL).await;
        assert!(r.is_empty());
        assert_eq!(r.sources_block(), "");
        let p = system_prompt(&r);
        assert!(p.contains("The brain is empty"));
        assert!(p.contains("Nothing in the vault matched"));
    }
