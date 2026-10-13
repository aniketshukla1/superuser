use super::*;

    /// A store of its own, and never the keychain: its entries are this user's, globally.
    fn place(tag: &str) -> Place {
        let dir = std::env::temp_dir().join(format!("su-secrets-{tag}-{}", crate::core::inbox::short_id()));
        fs::create_dir_all(&dir).unwrap();
        Place { dir, keychain: false, migrate: true, read_only: false }
    }

    #[test]
    fn what_is_saved_reads_back_and_is_not_on_disk_as_written() {
        let p = place("roundtrip");
        assert!(set_at(&p, "provider_openai", "sk-test-abcdef0123456789"));
        assert!(set_at(&p, "integration_github", "ghp_example"));
        assert_eq!(get_at(&p, "provider_openai").as_deref(), Some("sk-test-abcdef0123456789"));
        let on_disk = fs::read_to_string(p.store()).unwrap();
        assert!(!on_disk.contains("sk-test") && !on_disk.contains("provider_openai"), "neither the key nor what it is for: {on_disk}");
        assert!(clear_at(&p, "provider_openai"));
        assert_eq!(get_at(&p, "provider_openai"), None);
        assert_eq!(get_at(&p, "integration_github").as_deref(), Some("ghp_example"), "clearing one keeps the rest");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            for f in [p.store(), p.key_file()] {
                assert_eq!(fs::metadata(&f).unwrap().permissions().mode() & 0o777, 0o600, "{}", f.display());
            }
        }
        let _ = fs::remove_dir_all(&p.dir);
    }

    #[test]
    fn a_plaintext_store_is_converted_where_it_is_found() {
        let p = place("migrate");
        crate::core::store::write_atomic_private(&p.store(), r#"{"anthropic_api_key":"sk-ant-old","account_github":"@octocat"}"#).unwrap();
        assert_eq!(get_at(&p, "anthropic_api_key").as_deref(), Some("sk-ant-old"));
        assert!(!fs::read_to_string(p.store()).unwrap().contains("sk-ant-old"), "the plaintext is gone once read");
        assert_eq!(read_at(&p).unwrap().1, Sealing::Encrypted);
        assert_eq!(get_at(&p, "account_github").as_deref(), Some("@octocat"));

        // A scratch instance leaves the shared file exactly as it found it.
        let scratch = Place { migrate: false, ..place("scratch") };
        crate::core::store::write_atomic_private(&scratch.store(), r#"{"anthropic_api_key":"sk-ant-old"}"#).unwrap();
        assert_eq!(get_at(&scratch, "anthropic_api_key").as_deref(), Some("sk-ant-old"));
        assert!(fs::read_to_string(scratch.store()).unwrap().contains("sk-ant-old"));
        assert!(!scratch.key_file().exists());
        let _ = fs::remove_dir_all(&p.dir);
        let _ = fs::remove_dir_all(&scratch.dir);
    }

    #[test]
    fn a_store_that_wont_open_is_locked_and_nothing_is_written_over_it() {
        let p = place("locked");
        assert!(set_at(&p, "provider_openai", "sk-one"));
        assert!(set_at(&p, "provider_groq", "gsk-two"));
        let sealed = fs::read_to_string(p.store()).unwrap();

        // The key goes missing.
        fs::remove_file(p.key_file()).unwrap();
        assert!(read_at(&p).unwrap_err().contains("key that opens it is missing"));
        assert!(!set_at(&p, "provider_mistral", "new"), "saving one key must not replace the two it couldn't read");
        assert!(!clear_at(&p, "provider_openai"));
        assert_eq!(fs::read_to_string(p.store()).unwrap(), sealed, "the sealed store is untouched");
        assert!(write_at(&p, &Map::new()).is_err(), "and no new key is minted over it");
        let app = Place::app();
        assert!(app.read_only && !app.migrate && !app.keychain, "a test run can't change the real store");
        assert!(!p.key_file().exists());

        // A different key, or an altered file, is the same: locked, not empty.
        crate::core::store::write_atomic_private(&p.key_file(), &B64.encode([7u8; 32])).unwrap();
        assert!(read_at(&p).unwrap_err().contains("doesn't open it"));
        assert_eq!(get_at(&p, "provider_groq"), None);

        // The way out: the locked file is set aside — kept, not deleted — and keys save again.
        let kept = set_aside_at(&p).unwrap();
        assert_eq!(fs::read_to_string(&kept).unwrap(), sealed);
        assert!(set_at(&p, "provider_mistral", "new"));
        assert_eq!(get_at(&p, "provider_mistral").as_deref(), Some("new"));
        assert!(set_aside_at(&p).unwrap_err().contains("nothing to set aside"), "a store that opens is never moved");
        let _ = fs::remove_dir_all(&p.dir);
    }

    #[test]
    fn a_sealed_store_opens_only_with_its_own_key_and_context() {
        let key = [1u8; 32];
        let sealed: Map<String, Value> = serde_json::from_str::<Value>(&seal(&key, b"{}").unwrap()).unwrap().as_object().cloned().unwrap();
        assert_eq!(open(&key, &sealed).unwrap(), b"{}");
        assert!(open(&[2u8; 32], &sealed).is_err());
        let mut altered = sealed.clone();
        let mut data = B64.decode(altered["data"].as_str().unwrap()).unwrap();
        data[0] ^= 1;
        altered.insert("data".into(), Value::String(B64.encode(data)));
        assert!(open(&key, &altered).is_err(), "a changed byte is refused, not decrypted to garbage");
        // Two seals of the same thing differ: every write takes a fresh nonce.
        assert_ne!(seal(&key, b"{}").unwrap(), seal(&key, b"{}").unwrap());
    }

    /// Where the person's accounts are kept: the app's store, except in a
    /// scratch instance, which has its own in a subfolder of its folder and no keychain mirror —
    /// unless it was launched to share them.
    #[test]
    fn a_scratch_instance_keeps_its_accounts_in_its_own_folder() {
        let app = || Place { dir: PathBuf::from("/the/app"), keychain: true, migrate: true, read_only: false };
        let own = accounts_place_for(Some(PathBuf::from("/scratch/run")), false, app());
        assert_eq!(own.dir, PathBuf::from("/scratch/run/accounts"));
        assert!(!own.keychain, "the keychain's entries are the real app's");
        assert!(!own.read_only);
        assert_eq!(accounts_place_for(Some(PathBuf::from("/scratch/run")), true, app()).dir, PathBuf::from("/the/app"));
        assert_eq!(accounts_place_for(None, false, app()).dir, PathBuf::from("/the/app"));
        assert_eq!(accounts_place_for(None, true, app()).dir, PathBuf::from("/the/app"));
    }

    /// What a scratch instance does with accounts never reaches the real ones: it can't read
    /// them, what it saves lands in its own store, and disconnecting there disconnects nothing.
    #[test]
    fn a_scratch_store_neither_reads_nor_changes_the_real_accounts() {
        let real = place("real-accounts");
        assert!(set_at(&real, "integration_slack", "xoxb-real"));
        assert!(set_at(&real, "agent_token_assistant", "real-agent-token"));
        let scratch_dir = std::env::temp_dir().join(format!("su-scratch-{}", crate::core::inbox::short_id()));
        let scratch = accounts_place_for(Some(scratch_dir.clone()), false, Place { dir: real.dir.clone(), keychain: false, migrate: true, read_only: false });

        // Not in its file, and it doesn't look in the keychain — where this machine may well
        // hold a real `integration_slack` — so there is nothing to read.
        assert_eq!(get_in(&scratch, "integration_slack"), None);
        assert_eq!(get_in(&scratch, "agent_token_assistant"), None);

        assert!(set_in(&scratch, "integration_slack", "xoxb-scratch"));
        assert_eq!(get_in(&scratch, "integration_slack").as_deref(), Some("xoxb-scratch"));
        assert_eq!(get_at(&real, "integration_slack").as_deref(), Some("xoxb-real"), "saving in a scratch instance replaced the real token");
        assert!(scratch_dir.join("accounts").join("secrets.json").exists());

        assert!(clear_in(&scratch, "integration_slack"));
        assert!(clear_in(&scratch, "agent_token_assistant"));
        assert_eq!(get_at(&real, "integration_slack").as_deref(), Some("xoxb-real"), "disconnecting in a scratch instance disconnected the real account");
        assert_eq!(get_at(&real, "agent_token_assistant").as_deref(), Some("real-agent-token"));
        let _ = fs::remove_dir_all(&scratch_dir);
        let _ = fs::remove_dir_all(&real.dir);
    }

    /// Every account credential is read and written through `accounts_place()`, so a new one
    /// can't quietly go to the real store from a scratch instance.
    #[test]
    fn every_account_credential_goes_through_the_accounts_store() {
        let code = crate::test_support::module_source("core/secrets").split("#[cfg(test)]").next().unwrap();
        let uses: Vec<&str> = code.lines().filter(|l| ["&integ_key(", "&agent_key(", "&account_key("].iter().any(|k| l.contains(k))).collect();
        assert!(uses.len() >= 8, "found only {} uses — has the naming changed?", uses.len());
        for line in uses {
            assert!(line.contains("accounts_place()"), "an account credential outside the accounts store: {}", line.trim());
        }
    }
