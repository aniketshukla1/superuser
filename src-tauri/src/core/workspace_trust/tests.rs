use super::*;

    /// A directory of its own for every caller.
    ///
    /// Counter, not a timestamp: these tests run concurrently and the clock is not
    /// fine-grained enough to guarantee two threads get different nanoseconds. Two
    /// tests sharing a directory shows up as one of them failing to delete it,
    /// intermittently, which is the most expensive kind of test to own.
    fn scratch() -> PathBuf {
        use std::sync::atomic::{AtomicU32, Ordering};
        static N: AtomicU32 = AtomicU32::new(0);
        let p = std::env::temp_dir().join(format!(
            "wstrust-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(p.join(".superuser")).unwrap();
        p
    }

    fn declare(ws: &Path, json: &str) {
        std::fs::write(ws.join(CONFIG_PATH), json).unwrap();
    }

    #[test]
    fn the_row_serializes_under_the_names_the_card_reads() {
        // `WorkspaceCommandTrust` in api.ts reads exactly these keys. The card was
        // shipped against this contract long before anything served it, so the
        // field names are the contract — renaming one silently empties the card.
        let ws = scratch();
        declare(&ws, r#"{"allow_commands":["cargo test"]}"#);
        let data = scratch();
        let store = TrustStore::load(&data);
        store.set(&ws.to_string_lossy(), true).unwrap();
        let v = serde_json::to_value(&store.list()[0]).unwrap();
        for key in ["workspace", "requested_commands", "trusted", "required", "exists"] {
            assert!(v.get(key).is_some(), "the card reads `{key}`, which is missing from {v}");
        }
        assert!(v["requested_commands"].is_array());
        assert_eq!(v["trusted"], true);
        let _ = std::fs::remove_dir_all(&ws);
        let _ = std::fs::remove_dir_all(&data);
    }

    #[test]
    fn an_untrusted_project_declares_into_the_void() {
        // The rule the whole module exists for: cloning a repo must not be the act
        // that grants it permission to run commands.
        let ws = scratch();
        let data = scratch();
        declare(&ws, r#"{"allow_commands":["cargo test","git status"]}"#);
        assert_eq!(declared_commands(&ws).len(), 2, "the card can still SHOW what it asks for");
        assert!(allowances(&data, &ws).is_empty(), "but nothing is granted");
        let _ = std::fs::remove_dir_all(&ws);
        let _ = std::fs::remove_dir_all(&data);
    }

    #[test]
    fn trusting_the_folder_turns_the_declaration_on_and_revoking_turns_it_off() {
        let ws = scratch();
        let data = scratch();
        declare(&ws, r#"{"allow_commands":["cargo test"]}"#);
        let store = TrustStore::load(&data);

        store.set(&ws.to_string_lossy(), true).unwrap();
        assert_eq!(allowances(&data, &ws), vec!["cargo test".to_string()]);

        store.set(&ws.to_string_lossy(), false).unwrap();
        assert!(allowances(&data, &ws).is_empty());
        let _ = std::fs::remove_dir_all(&ws);
        let _ = std::fs::remove_dir_all(&data);
    }

    #[test]
    fn trust_follows_the_path_not_the_config_it_had_when_trusted() {
        // Re-prompting on every edit trains people to click through the prompt.
        let ws = scratch();
        let data = scratch();
        declare(&ws, r#"{"allow_commands":["cargo test"]}"#);
        TrustStore::load(&data).set(&ws.to_string_lossy(), true).unwrap();

        declare(&ws, r#"{"allow_commands":["cargo test","cargo build"]}"#);
        assert_eq!(allowances(&data, &ws).len(), 2, "the project's later edits are accepted");
        let _ = std::fs::remove_dir_all(&ws);
        let _ = std::fs::remove_dir_all(&data);
    }

    #[test]
    fn one_folder_is_one_decision_however_it_is_spelled() {
        let ws = scratch();
        let data = scratch();
        let store = TrustStore::load(&data);
        store.set(&ws.to_string_lossy(), true).unwrap();
        // The same directory reached through a symlink is the same directory.
        let link = scratch().with_extension("link");
        let _ = std::fs::remove_file(&link);
        if std::os::unix::fs::symlink(&ws, &link).is_ok() {
            assert!(store.is_trusted(&link.to_string_lossy()), "a symlink is not a second decision");
            let _ = std::fs::remove_file(&link);
        }
        assert_eq!(store.list().len(), 1);
        let _ = std::fs::remove_dir_all(&ws);
        let _ = std::fs::remove_dir_all(&data);
    }

    #[test]
    fn a_project_cannot_declare_an_unbounded_list() {
        let ws = scratch();
        let many: Vec<String> = (0..200).map(|i| format!("cmd{i}")).collect();
        declare(&ws, &serde_json::json!({ "allow_commands": many }).to_string());
        assert_eq!(declared_commands(&ws).len(), MAX_DECLARED);
        let _ = std::fs::remove_dir_all(&ws);
    }

    #[test]
    fn a_broken_or_missing_config_declares_nothing() {
        let ws = scratch();
        assert!(declared_commands(&ws).is_empty(), "no file at all");
        declare(&ws, "{ not json");
        assert!(declared_commands(&ws).is_empty(), "unparseable");
        declare(&ws, r#"{"allow_commands":["  ", ""]}"#);
        assert!(declared_commands(&ws).is_empty(), "blank entries are not commands");
        let _ = std::fs::remove_dir_all(&ws);
    }

    #[test]
    fn the_card_can_describe_a_folder_nobody_has_trusted_yet() {
        let ws = scratch();
        let data = scratch();
        declare(&ws, r#"{"allow_commands":["cargo test"]}"#);
        let s = TrustStore::load(&data).status(&ws.to_string_lossy());
        assert!(!s.trusted);
        assert!(s.required, "it does ask for something");
        assert!(s.exists);
        assert_eq!(s.requested_commands, vec!["cargo test".to_string()]);
        let _ = std::fs::remove_dir_all(&ws);
        let _ = std::fs::remove_dir_all(&data);
    }

    #[test]
    fn a_deleted_project_stays_revocable() {
        // Canonicalizing a missing path fails; keeping the stored spelling is what
        // lets the user revoke the row the card is showing them.
        let ws = scratch();
        let data = scratch();
        let store = TrustStore::load(&data);
        store.set(&ws.to_string_lossy(), true).unwrap();
        let key = store.list()[0].workspace.clone();
        std::fs::remove_dir_all(&ws).unwrap();

        let rows = store.list();
        assert_eq!(rows.len(), 1);
        assert!(!rows[0].exists, "the card says the folder is gone");
        store.set(&key, false).unwrap();
        assert!(store.list().is_empty(), "and it can still be revoked");
        let _ = std::fs::remove_dir_all(&data);
    }
