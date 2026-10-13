use super::*;

    pub(super) fn scratch() -> PathBuf {
        use std::sync::atomic::{AtomicU32, Ordering};
        static N: AtomicU32 = AtomicU32::new(0);
        let p = std::env::temp_dir()
            .join(format!("pinstall-{}-{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst)));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    pub(super) fn manifest(id: &str, tools: &str) -> String {
        format!("---\nid: {id}\nname: {id}\ntagline: does {id} things\ndescription: an assistant that does {id} things, at some length\ntools: [{tools}]\n---\nYou are {id}.")
    }

    #[tokio::test]
    pub(super) async fn a_folder_of_manifests_installs_and_arrives_switched_off() {
        let src = scratch();
        std::fs::write(src.join("ops.md"), manifest("acme-ops", "files, shell")).unwrap();
        let data = scratch();

        let cards = install(&data, Source::Dir(src.to_string_lossy().into_owned())).await.unwrap();
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].id, "acme-ops");
        assert!(data.join("personas/acme-ops.md").exists(), "copied into the managed area");

        // The safety property. An assistant someone else wrote must not be startable
        // until the user has read the card and turned it on.
        let reg = PersonaRegistry::load(&data);
        assert!(reg.manifest("acme-ops").is_some(), "installed");
        assert!(!reg.is_enabled("acme-ops"), "and off until the user says otherwise");
        assert!(!reg.sidebar().iter().any(|p| p.id == "acme-ops"), "so it is not in the picker");

        let _ = std::fs::remove_dir_all(&src);
        let _ = std::fs::remove_dir_all(&data);
    }

    #[tokio::test]
    pub(super) async fn the_card_says_where_it_came_from_not_where_it_landed() {
        // "From /Users/.../Application Support/.../acme-ops.md" tells the user
        // nothing. Months later the question is who wrote this, and the answer is
        // the source they typed in.
        let src = scratch();
        std::fs::write(src.join("ops.md"), manifest("acme-ops", "files")).unwrap();
        let data = scratch();

        let cards = install(&data, Source::Dir(src.to_string_lossy().into_owned())).await.unwrap();
        let from = cards[0].source.clone().unwrap_or_default();
        assert!(from.starts_with(&src.to_string_lossy().to_string()), "got: {from}");
        assert!(!from.contains("personas/acme-ops.md") || from.starts_with(&src.to_string_lossy().to_string()));

        let _ = std::fs::remove_dir_all(&src);
        let _ = std::fs::remove_dir_all(&data);
    }

    #[tokio::test]
    pub(super) async fn the_card_says_what_it_can_do_rather_than_what_it_claims() {
        // Risk is DERIVED from the capability list: a manifest that got to describe
        // its own risk would describe it as none.
        let src = scratch();
        std::fs::write(src.join("a.md"), manifest("runner", "files, shell")).unwrap();
        std::fs::write(src.join("b.md"), manifest("reader", "search")).unwrap();
        let data = scratch();

        let cards = install(&data, Source::Dir(src.to_string_lossy().into_owned())).await.unwrap();
        let runner = cards.iter().find(|c| c.id == "runner").unwrap();
        let reader = cards.iter().find(|c| c.id == "reader").unwrap();
        assert!(runner.risk.contains(&crate::core::agent::RiskClass::Exec), "shell is exec, and the card must say so");
        assert!(!reader.risk.contains(&crate::core::agent::RiskClass::Exec));
        assert!(runner.tools.contains(&"shell".to_string()));

        let _ = std::fs::remove_dir_all(&src);
        let _ = std::fs::remove_dir_all(&data);
    }

    #[tokio::test]
    pub(super) async fn one_bad_manifest_installs_nothing() {
        // Half an install is the worst outcome: the user is told it failed and left
        // holding assistants they never saw a card for.
        let src = scratch();
        std::fs::write(src.join("good.md"), manifest("fine", "files")).unwrap();
        // Frontmatter that opens and then goes wrong — a file trying to be an
        // assistant, not the prose that `retain` skips.
        std::fs::write(src.join("bad.md"), "---\ntools: [unclosed\n---\nbody").unwrap();
        let data = scratch();

        assert!(install(&data, Source::Dir(src.to_string_lossy().into_owned())).await.is_err());
        assert!(!data.join("personas/fine.md").exists(), "the good one did not land either");

        let _ = std::fs::remove_dir_all(&src);
        let _ = std::fs::remove_dir_all(&data);
    }

    #[tokio::test]
    pub(super) async fn a_manifest_cannot_shadow_a_builtin() {
        // It would sit on disk being ignored while the user believed they had
        // installed something — the built-in's manifest is compiled in and wins.
        let src = scratch();
        std::fs::write(src.join("x.md"), manifest("security", "files")).unwrap();
        let data = scratch();

        let err = install(&data, Source::Dir(src.to_string_lossy().into_owned())).await.unwrap_err();
        assert!(err.contains("built-in"), "got: {err}");

        let _ = std::fs::remove_dir_all(&src);
        let _ = std::fs::remove_dir_all(&data);
    }

    #[tokio::test]
    pub(super) async fn a_readme_is_documentation_not_an_assistant() {
        let src = scratch();
        std::fs::write(src.join("README.md"), "# Our personas\n\nInstall them with…").unwrap();
        std::fs::write(src.join("ops.md"), manifest("acme-ops", "files")).unwrap();
        let data = scratch();

        let cards = install(&data, Source::Dir(src.to_string_lossy().into_owned())).await.unwrap();
        assert_eq!(cards.len(), 1, "the README must not fail the install or become an assistant");

        let _ = std::fs::remove_dir_all(&src);
        let _ = std::fs::remove_dir_all(&data);
    }

    #[tokio::test]
    pub(super) async fn prose_beside_the_bundle_is_skipped_but_a_broken_manifest_is_not() {
        // A source is often a whole repo. CONTRIBUTING.md must not fail the install;
        // a file that opens `---` and then goes wrong was trying to be an assistant,
        // and installing the rest while saying nothing is how you end up with two of
        // three and a success message.
        let src = scratch();
        std::fs::write(src.join("CONTRIBUTING.md"), "# How to contribute\n\nOpen a PR.").unwrap();
        std::fs::write(src.join("ops.md"), manifest("acme-ops", "files")).unwrap();
        let data = scratch();
        let cards = install(&data, Source::Dir(src.to_string_lossy().into_owned())).await.unwrap();
        assert_eq!(cards.len(), 1, "prose is not an assistant");

        std::fs::write(src.join("broken.md"), "---\nname: [unclosed\n---\nbody").unwrap();
        let data2 = scratch();
        assert!(
            install(&data2, Source::Dir(src.to_string_lossy().into_owned())).await.is_err(),
            "a manifest that tried and failed is an error, not a skip"
        );
        assert!(!data2.join("personas/acme-ops.md").exists(), "and nothing lands");

        let _ = std::fs::remove_dir_all(&src);
        let _ = std::fs::remove_dir_all(&data);
        let _ = std::fs::remove_dir_all(&data2);
    }

    #[tokio::test]
    pub(super) async fn a_bundle_can_live_in_a_personas_subfolder() {
        let src = scratch();
        std::fs::create_dir_all(src.join("personas")).unwrap();
        std::fs::write(src.join("README.md"), "# repo").unwrap();
        std::fs::write(src.join("personas/ops.md"), manifest("acme-ops", "files")).unwrap();
        let data = scratch();

        let cards = install(&data, Source::Dir(src.to_string_lossy().into_owned())).await.unwrap();
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].id, "acme-ops");

        let _ = std::fs::remove_dir_all(&src);
        let _ = std::fs::remove_dir_all(&data);
    }

    #[tokio::test]
    pub(super) async fn an_empty_or_missing_folder_says_so_instead_of_succeeding_quietly() {
        let data = scratch();
        let empty = scratch();
        assert!(install(&data, Source::Dir(empty.to_string_lossy().into_owned())).await.is_err());
        assert!(install(&data, Source::Dir("/no/such/folder".into())).await.is_err());
        let _ = std::fs::remove_dir_all(&empty);
        let _ = std::fs::remove_dir_all(&data);
    }

    #[tokio::test]
    pub(super) async fn an_id_that_would_escape_the_personas_folder_is_refused() {
        let src = scratch();
        std::fs::write(src.join("evil.md"), manifest("../../etc/cron.d/x", "files")).unwrap();
        let data = scratch();
        assert!(install(&data, Source::Dir(src.to_string_lossy().into_owned())).await.is_err());
        let _ = std::fs::remove_dir_all(&src);
        let _ = std::fs::remove_dir_all(&data);
    }
