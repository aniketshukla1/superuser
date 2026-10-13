use super::super::*;

    //
    // Asserted through `providers`, because the rule this gets wrong is always the
    // same one: treating "cloud" as a synonym for Anthropic. A GPT/Codex/Groq pick
    // has to claim the cloud budget on ITS OWN provider's readiness, and `local_only`
    // has to beat all of them.

    #[test]
    pub(in crate::core::brain::tests) fn a_non_anthropic_cloud_model_is_not_mistaken_for_a_local_one() {
        use crate::core::providers;
        // Codex and OpenAI ids are cloud models whatever key is in the keychain...
        assert!(!providers::runs_locally("codex:gpt-5.6-sol"));
        assert!(!providers::runs_locally("openai:gpt-5.5"));
        assert!(!providers::runs_locally("claude-sonnet-5"));
        // ...while a bare Ollama tag is local even though it looks like provider:model.
        assert!(providers::runs_locally("gemma4:12b"));
        assert!(providers::runs_locally("qwen2.5:7b-instruct"));
    }

    #[test]
    pub(in crate::core::brain::tests) fn local_only_sends_the_question_to_the_local_model_whatever_was_picked() {
        use crate::core::providers;
        let mut s = Settings::default();
        s.local_only = true;
        s.ollama_model = "gemma4:12b".into();
        // The exact branch `ask` takes: a cloud pick under local_only is replaced by
        // the local model rather than merely losing its cloud budget.
        let chosen = "codex:gpt-5.6-sol".to_string();
        let swapped = if s.local_only && !providers::runs_locally(&chosen) {
            s.ollama_model.clone()
        } else {
            chosen
        };
        assert_eq!(swapped, "gemma4:12b");
    }

    #[test]
    pub(in crate::core::brain::tests) fn the_budget_follows_the_model_that_actually_runs() {
        use crate::core::providers;
        // A cloud model whose own provider is not configured falls back to the local
        // budget, because that is what will actually answer.
        let unready = "groq:llama-3.3-70b-versatile";
        let local = providers::runs_locally(unready) || !providers::cloud_model_ready(unready);
        assert_eq!(Budget::resolve(local), Budget::LOCAL, "an unconfigured provider must not claim the cloud budget");
        // And a local tag always takes the local budget.
        assert!(providers::runs_locally("gemma4:12b"));
        assert_eq!(Budget::resolve(true), Budget::LOCAL);
    }

    #[test]
    pub(in crate::core::brain::tests) fn the_progress_line_says_what_came_back() {
        // What the panel shows while the model is still thinking.
        let f = full_recall().found();
        assert!(f.contains("1 of 42 memories"), "{f}");
        assert!(f.contains("3 timeline entries"), "{f}");
        assert!(f.contains("1 knowledge pages"), "{f}");

        let all = vec![memory("Starred owner/alpha", "GitHub", "")];
        let census = census_of(&all, Path::new("/nonexistent"), Path::new("/nonexistent"));
        let r = Recall {
            memories: vec![],
            journal: vec![],
            pages: vec![],
            profile: String::new(),
            inventories: inventories(&all, "my GitHub", &census, Budget::CLOUD),
            census,
            degraded: None,
            budget: Budget::CLOUD,
        };
        assert_eq!(r.found(), "all 1 filed under GitHub");

        // An empty result still says something rather than rendering a blank row.
        let empty = Recall {
            memories: vec![],
            journal: vec![],
            pages: vec![],
            profile: String::new(),
            census: Census::default(),
            degraded: None,
            inventories: vec![],
            budget: Budget::LOCAL,
        };
        assert_eq!(empty.found(), "nothing matched");
    }

    /// The regression that hid a dead embedding model for months: recall fell back
    /// to keyword matching on every question and said nothing, so the only symptom
    /// was answers that were quietly worse than the vault could support.
    #[tokio::test]
    pub(in crate::core::brain::tests) async fn a_fallback_to_keyword_matching_is_reported_not_swallowed() {
        let (tmp, engine, settings) = seeded_vault();
        let vault = tmp.0.join("vault");
        // `seeded_vault` points Ollama at a dead port, so embedding cannot work —
        // exactly the shape of a misconfigured `embed_model`.
        let r = recall(&engine, &settings, &vault, &tmp.0, "Postgres", Budget::LOCAL).await;

        let why = r.degraded.as_deref().expect("a failed embed must be reported, not swallowed");
        assert!(!why.is_empty());
        // ...and it reaches the progress line the user actually reads.
        let found = r.found();
        assert!(found.contains("keyword matching only"), "{found}");
        // The answer is still produced — degrading beats failing.
        assert!(!r.memories.is_empty(), "keyword recall should still return the Postgres memory");
    }

    #[test]
    pub(in crate::core::brain::tests) fn a_working_search_says_nothing_about_fallbacks() {
        // The quiet path stays quiet: no reason, no noise on the progress row.
        let r = full_recall();
        assert!(r.degraded.is_none());
        assert!(!r.found().contains("keyword"), "{}", r.found());
    }

    #[test]
    pub(in crate::core::brain::tests) fn the_plan_names_the_model_before_any_work_happens() {
        // The point of `plan` being separate: the caller can say which model it is
        // about to wait on, instead of learning it from the answer.
        let mut s = Settings::default();
        s.local_only = true;
        s.ollama_model = "gemma4:12b".into();
        let p = plan(&s, Some("codex:gpt-5.6-luna"));
        assert_eq!(p.model, "gemma4:12b", "local_only must win, and the plan must say so up front");
        assert_eq!(p.budget, Budget::LOCAL);

        s.local_only = false;
        let p = plan(&s, Some("gemma4:12b"));
        assert_eq!(p.model, "gemma4:12b");
        assert_eq!(p.budget, Budget::LOCAL);
    }

    #[test]
    pub(in crate::core::brain::tests) fn the_census_counts_what_is_actually_on_disk() {
        let (tmp, engine, _) = seeded_vault();
        let vault = tmp.0.join("vault");
        let c = census(&engine, &vault, &tmp.0);
        assert_eq!(c.memories, 22);
        // BTreeMap order is byte order, so the capitalised category sorts first.
        assert_eq!(
            c.categories,
            vec![("GitHub".to_string(), 20), ("email".to_string(), 1), ("repo".to_string(), 1)]
        );
        assert_eq!(c.journal_days, 1);
        assert!(c.has_core_memory);
        assert_eq!(c.knowledge_pages, 1);
        assert!(!c.oldest.is_empty() && !c.newest.is_empty());
    }

    #[tokio::test]
    pub(in crate::core::brain::tests) async fn a_question_reaches_all_four_stores_on_disk() {
        let (tmp, engine, settings) = seeded_vault();
        let vault = tmp.0.join("vault");
        let r = recall(&engine, &settings, &vault, &tmp.0, "Postgres", Budget::CLOUD).await;

        assert!(!r.is_empty());
        assert!(
            r.memories.iter().any(|m| m.title == "Postgres migration plan"),
            "memories not read: {:?}",
            r.memories.iter().map(|m| &m.title).collect::<Vec<_>>()
        );
        assert!(!r.journal.is_empty(), "journal not read");
        assert!(r.pages.iter().any(|p| p.title == "Postgres"), "knowledge not read");

        // And the prompt the model actually gets carries every one of them plus the
        // exact counts.
        let p = system_prompt(&r);
        assert!(p.contains("October freeze"));
        assert!(p.contains("pgbouncer") || p.contains("short answers"));
        assert!(p.contains("Connection pooling notes"));
        assert!(p.contains("Memories: 22\n"));

        // The footer names real files, and only the ones that were retrieved.
        let s = r.sources_block();
        assert!(s.contains("Postgres migration plan (repo)"));
        assert!(!s.contains("Lunch order"), "an unretrieved memory was cited");
    }
