use super::*;

    #[tokio::test]
    pub(super) async fn a_source_offering_a_crowd_is_refused_rather_than_reviewed() {
        let src = scratch();
        for i in 0..(MAX_PERSONAS + 1) {
            std::fs::write(src.join(format!("p{i}.md")), manifest(&format!("bulk-{i}"), "files")).unwrap();
        }
        let data = scratch();
        let err = install(&data, Source::Dir(src.to_string_lossy().into_owned())).await.unwrap_err();
        assert!(err.contains("more than anyone reviews"), "got: {err}");
        assert!(!data.join("personas/bulk-0.md").exists());
        let _ = std::fs::remove_dir_all(&src);
        let _ = std::fs::remove_dir_all(&data);
    }

    #[test]
    pub(super) fn the_shapes_people_actually_paste() {
        let (o, r, s) = repo_of("https://github.com/acme/ops-persona").unwrap();
        assert_eq!((o.as_str(), r.as_str(), s), ("acme", "ops-persona", None));
        assert_eq!(repo_of("acme/ops-persona").unwrap().0, "acme");
        assert_eq!(repo_of("https://github.com/acme/bundle.git").unwrap().1, "bundle");
        // A subfolder in the address bar is how a repo ships several bundles.
        let (_, _, sub) = repo_of("https://github.com/acme/bundle/tree/main/finance").unwrap();
        assert_eq!(sub.as_deref(), Some("finance"));

        assert!(repo_of("").is_err());
        assert!(repo_of("acme").is_err());
        // Plain HTTP is refused rather than downgraded to: the whole payload is
        // instructions a model will obey, and anyone on the path could rewrite it.
        assert!(repo_of("http://github.com/acme/x").unwrap_err().contains("plain HTTP"));
    }
