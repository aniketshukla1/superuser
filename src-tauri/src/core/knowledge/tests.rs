use super::*;

    fn scratch() -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(0);
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let p = std::env::temp_dir().join(format!("bba-knowledge-{}-{}", nanos, N.fetch_add(1, Ordering::SeqCst)));
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn slugs_are_stable_and_filesystem_safe() {
        assert_eq!(slugify("Postgres vs MySQL"), "postgres-vs-mysql");
        assert_eq!(slugify("  Trailing / slashes  "), "trailing-slashes");
        assert_eq!(slugify("../../etc/passwd"), "etc-passwd", "traversal cannot survive slugging");
        assert_eq!(slugify("!!!"), "page");
        assert_eq!(slugify("Café ☕"), "caf");
    }

    #[test]
    fn wikilinks_are_extracted_deduplicated_and_labelled() {
        let body = "See [[Postgres]] and [[Indexing|how indexes work]]. Again [[Postgres]].";
        assert_eq!(wikilinks(body), vec!["Postgres", "Indexing"]);
        assert!(wikilinks("no links here").is_empty());
        assert!(wikilinks("unterminated [[oops").is_empty());
    }

    #[test]
    fn writing_the_same_topic_twice_improves_one_page() {
        let v = scratch();
        let k = Knowledge::new(&v);
        k.write("Postgres", "concepts", "A relational database.").unwrap();
        let p = k.write("Postgres", "concepts", "A relational database. We use it for new services.").unwrap();

        assert_eq!(k.list().len(), 1, "upsert, not append");
        assert_eq!(p.slug, "postgres");
        assert!(p.body.contains("new services"));
        assert_eq!(p.title, "Postgres", "the title survives the round trip");
        fs::remove_dir_all(&v).ok();
    }

    #[test]
    fn moving_a_page_between_categories_leaves_no_duplicate() {
        let v = scratch();
        let k = Knowledge::new(&v);
        k.write("Postgres", "concepts", "…").unwrap();
        let moved = k.write("Postgres", "tools", "…").unwrap();
        assert_eq!(moved.category, "tools");
        assert_eq!(k.list().len(), 1, "the old category's file is removed");
        assert_eq!(k.find("postgres").unwrap().category, "tools");
        fs::remove_dir_all(&v).ok();
    }

    #[test]
    fn the_knowledge_base_is_invisible_to_the_vault_listing() {
        let v = scratch();
        Knowledge::new(&v).write("Postgres", "concepts", "…").unwrap();
        // The vault lists notes non-recursively; a page must not also appear there.
        assert!(crate::core::memory::MemoryEngine::new(v.clone()).list().is_empty());
        fs::remove_dir_all(&v).ok();
    }

    #[test]
    fn the_index_regenerates_and_scaffolding_is_not_knowledge() {
        let v = scratch();
        let k = Knowledge::new(&v);
        k.write("Postgres", "concepts", "A relational database.").unwrap();
        k.write("Ada Lovelace", "people", "First programmer.").unwrap();

        let index = fs::read_to_string(k.root.join("index.md")).unwrap();
        assert!(index.contains("## concepts") && index.contains("## people"));
        assert!(index.contains("[[Postgres]] — A relational database."));
        // index.md and log.md live in the tree but are generated, not pages.
        assert_eq!(k.list().len(), 2);
        assert!(!k.list().iter().any(|p| p.slug == "index" || p.slug == "log"));

        // Deleting rewrites the index rather than leaving a dangling entry.
        assert!(k.delete("postgres"));
        assert!(!fs::read_to_string(k.root.join("index.md")).unwrap().contains("[[Postgres]]"));
        fs::remove_dir_all(&v).ok();
    }

    #[test]
    fn every_change_is_logged() {
        let v = scratch();
        let k = Knowledge::new(&v);
        k.write("Postgres", "concepts", "…").unwrap();
        k.delete("postgres");
        let log = fs::read_to_string(k.root.join("log.md")).unwrap();
        assert!(log.contains("wrote [[Postgres]] in concepts"));
        assert!(log.contains("removed Postgres"));
        fs::remove_dir_all(&v).ok();
    }

    #[test]
    fn search_puts_the_page_about_a_thing_above_one_that_mentions_it() {
        let v = scratch();
        let k = Knowledge::new(&v);
        k.write("Postgres", "concepts", "A relational database we use.").unwrap();
        k.write("Deployment", "projects", "We deploy the app; it talks to postgres.").unwrap();

        let hits = k.search("postgres", 5);
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].title, "Postgres", "title match wins");
        assert!(k.search("tarragon", 5).is_empty());
        assert!(k.search("a", 5).is_empty(), "short terms are dropped");
        fs::remove_dir_all(&v).ok();
    }

    #[test]
    fn the_graph_is_built_from_real_links_not_guesses() {
        let v = scratch();
        let k = Knowledge::new(&v);
        k.write("Postgres", "concepts", "Related: [[Indexing]] and [[Nonexistent Page]].").unwrap();
        k.write("Indexing", "concepts", "See [[Postgres]].").unwrap();

        let g = k.graph();
        let nodes = g["nodes"].as_array().unwrap();
        let edges = g["edges"].as_array().unwrap();
        assert_eq!(nodes.iter().filter(|n| n["kind"] == "page").count(), 2);
        assert_eq!(nodes.iter().filter(|n| n["kind"] == "category").count(), 1, "one shared category");
        // A link to a page nobody has written yet is not drawn as a phantom node.
        assert!(!nodes.iter().any(|n| n["label"] == "Nonexistent Page"));
        let page_edges = edges.iter().filter(|e| e["s"].as_str().unwrap().starts_with("k:")).count();
        assert_eq!(page_edges, 2, "one edge each way between the two real pages");
        fs::remove_dir_all(&v).ok();
    }

    #[test]
    fn a_body_that_repeats_the_title_does_not_produce_two_headings() {
        let v = scratch();
        let k = Knowledge::new(&v);
        // Measured with a real model: asked to write a page it opens with the title
        // as an H1, and the file ended up with the heading twice.
        let p = k
            .write("Rust ownership", "concepts", "# Rust Ownership\n\nEach value has one owner.\n\n## Borrowing\n\nReferences borrow.")
            .unwrap();
        assert!(p.body.starts_with("Each value has one owner."), "got: {:?}", p.body);
        assert_eq!(fs::read_to_string(k.page_path("concepts", "rust-ownership")).unwrap().matches("# Rust").count(), 1);
        // The index shows prose, not a heading.
        assert!(fs::read_to_string(k.root.join("index.md")).unwrap().contains("— Each value has one owner."));
        // Sub-headings inside the body are untouched.
        assert!(p.body.contains("## Borrowing"));
        fs::remove_dir_all(&v).ok();
    }

    #[test]
    fn an_untitled_page_is_refused() {
        let v = scratch();
        assert!(Knowledge::new(&v).write("   ", "concepts", "body").is_err());
        fs::remove_dir_all(&v).ok();
    }
