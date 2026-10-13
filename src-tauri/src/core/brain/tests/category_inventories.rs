//! category inventories

use super::*;

    //
    // The regression these exist for: asked which repositories they had starred, a
    // 125-memory vault holding 20 such memories answered "no record of any", because
    // the eight retrieval slots went elsewhere. Counting was already exact; listing
    // was not.

    pub(super) fn github_census() -> Census {
        Census {
            categories: vec![
                ("Chat".into(), 5),
                ("GitHub".into(), 117),
                ("Notion".into(), 1),
            ],
            memories: 123,
            ..Default::default()
        }
    }

    #[test]
    pub(super) fn a_category_named_in_the_question_is_matched_on_a_word_boundary() {
        let c = github_census();
        let m = |q: &str| matched_categories(q, &c);
        assert_eq!(m("which GitHub repos have I starred?"), vec!["GitHub"]);
        assert_eq!(m("lowercase github works too"), vec!["GitHub"]);
        // Plural, and more than one category at once (census order).
        assert_eq!(m("my Notions and chats"), vec!["Chat", "Notion"]);
        // Not a match: the name has to stand as its own word.
        assert!(m("the chatter was constant").is_empty());
        assert!(m("githubbed it").is_empty());
        // Most questions name no category, and pay nothing for this.
        assert!(m("what did I decide about Postgres?").is_empty());
    }

    #[test]
    pub(super) fn a_matched_category_is_listed_in_full_and_says_so() {
        let all: Vec<Memory> = vec![
            memory("Starred owner/alpha", "GitHub", ""),
            memory("Starred owner/beta", "GitHub", ""),
            memory("Lunch", "Chat", ""),
        ];
        let census = census_of(&all, Path::new("/nonexistent"), Path::new("/nonexistent"));
        let inv = inventories(&all, "which GitHub repos did I star?", &census, Budget::CLOUD);
        assert_eq!(inv.len(), 1);
        assert_eq!(inv[0].category, "GitHub");
        assert_eq!(inv[0].total, 2);
        assert!(inv[0].complete());
        assert!(inv[0].titles.contains(&"Starred owner/alpha".to_string()));
        assert!(inv[0].titles.contains(&"Starred owner/beta".to_string()));
        // The unmatched category is not listed — this is targeted, not a vault dump.
        assert!(!inv.iter().any(|i| i.category == "Chat"));

        let r = Recall {
            memories: vec![],
            journal: vec![],
            pages: vec![],
            profile: String::new(),
            census,
            inventories: inv,
            degraded: None,
            budget: Budget::CLOUD,
        };
        let block = r.inventory_block();
        assert!(block.contains("all 2 memories filed under \"GitHub\""));
        assert!(block.contains("COMPLETE"));
        assert!(block.contains("- Starred owner/alpha"));
        // An inventory alone is material, so the answer is not treated as empty.
        assert!(!r.is_empty());
        assert!(r.sources_block().contains("all 2 memories filed under GitHub"));
    }

    #[test]
    pub(super) fn a_capped_inventory_is_labelled_incomplete_rather_than_passed_off_as_whole() {
        // Newest first, so a capped list keeps the recent half.
        let all: Vec<Memory> = (0..10)
            .map(|i| {
                let mut m = memory(&format!("Repo {i}"), "GitHub", "");
                m.date = format!("2026-08-{:02}", i + 1);
                m
            })
            .collect();
        let census = census_of(&all, Path::new("/nonexistent"), Path::new("/nonexistent"));
        let budget = Budget { inventory_titles: 3, ..Budget::CLOUD };
        let inv = inventories(&all, "list my GitHub repos", &census, budget);

        assert_eq!(inv[0].titles.len(), 3);
        assert_eq!(inv[0].total, 10);
        assert!(!inv[0].complete());
        assert_eq!(inv[0].titles, vec!["Repo 9", "Repo 8", "Repo 7"]);

        let r = Recall {
            memories: vec![],
            journal: vec![],
            pages: vec![],
            profile: String::new(),
            census,
            inventories: inv,
            degraded: None,
            budget: Budget::CLOUD,
        };
        let block = r.inventory_block();
        assert!(block.contains("3 most recent of 10"));
        assert!(block.contains("INCOMPLETE"));
        // "list is COMPLETE" and not merely "COMPLETE", which is a substring of the
        // word that says the opposite.
        assert!(!block.contains("list is COMPLETE"), "a trimmed list must not claim completeness");
        assert!(r.sources_block().contains("3 of 10 memories filed under GitHub (partial list)"));
    }

    #[test]
    pub(super) fn several_matched_categories_share_one_title_budget() {
        let mut all: Vec<Memory> = (0..5).map(|i| memory(&format!("c{i}"), "Chat", "")).collect();
        all.extend((0..5).map(|i| memory(&format!("g{i}"), "GitHub", "")));
        let census = census_of(&all, Path::new("/nonexistent"), Path::new("/nonexistent"));
        let budget = Budget { inventory_titles: 7, ..Budget::CLOUD };
        let inv = inventories(&all, "my chats and my GitHub", &census, budget);
        // Two matches, and together they never exceed what one was allowed.
        assert_eq!(inv.len(), 2);
        let listed: usize = inv.iter().map(|i| i.titles.len()).sum();
        assert_eq!(listed, 7);
        // The one that ran out of room is honest about it.
        assert!(inv.iter().any(|i| !i.complete()));
    }

    #[test]
    pub(super) fn the_prompt_tells_the_model_an_inventory_outranks_the_excerpts() {
        let all = vec![memory("Starred owner/alpha", "GitHub", "")];
        let census = census_of(&all, Path::new("/nonexistent"), Path::new("/nonexistent"));
        let r = Recall {
            memories: vec![],
            journal: vec![],
            pages: vec![],
            profile: String::new(),
            inventories: inventories(&all, "which GitHub repos?", &census, Budget::CLOUD),
            census,
            degraded: None,
            budget: Budget::CLOUD,
        };
        let p = system_prompt(&r);
        assert!(p.contains("INVENTORY"));
        assert!(p.contains("Starred owner/alpha"));
        // And the instruction that names the exact failure this fixes.
        assert!(p.contains("I have no record of that"));
    }

    #[tokio::test]
    pub(super) async fn an_inventory_displaces_the_excerpts_instead_of_stacking_on_them() {
        let (tmp, engine, settings) = seeded_vault();
        let vault = tmp.0.join("vault");

        let listed = recall(&engine, &settings, &vault, &tmp.0, "my GitHub repos", Budget::LOCAL).await;
        let plain = recall(&engine, &settings, &vault, &tmp.0, "the Postgres cutover", Budget::LOCAL).await;
        assert!(!listed.inventories.is_empty() && plain.inventories.is_empty());

        // The budget the excerpts were gathered under really did shrink...
        assert_eq!(listed.budget, Budget::LOCAL.displaced());
        assert_eq!(plain.budget, Budget::LOCAL);
        assert!(listed.budget.memories < Budget::LOCAL.memories);
        // ...but never to nothing: a category question can still hinge on what one
        // entry said.
        assert!(listed.budget.memories >= 2);
        // The inventory itself is untouched by the reduction — trimming the thing
        // that has to be exhaustive would defeat the entire feature.
        assert_eq!(listed.budget.inventory_titles, Budget::LOCAL.inventory_titles);
        assert!(listed.inventories[0].complete());

        // And the net effect on the prompt: listing 20 titles costs less than the
        // excerpt block it replaced, so the enumerative question is not the
        // expensive one.
        let (with, without) = (system_prompt(&listed).len(), system_prompt(&plain).len());
        assert!(
            with < without + 1200,
            "inventory added {} chars over the plain prompt — it is supposed to displace, not stack",
            with as i64 - without as i64
        );
    }
