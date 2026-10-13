use super::*;

    #[test]
    fn syncs_agrees_with_the_arms_sync_inner_runs() {
        // Read from source rather than by calling `sync_one`: a test that syncs would
        // use whatever token this machine's keychain holds. A named arm that returns
        // `Err(` straight away is a refusal (Prometheus, WhatsApp); any other runs a sync.
        let src = crate::test_support::module_source("integrations");
        let body = &src[src.find("async fn sync_inner").expect("sync_inner renamed")..];
        let body = &body[..body.find("\n}\n").expect("unterminated sync_inner")];
        let mut arms = 0;
        for line in body.lines().map(str::trim_start).filter(|l| l.starts_with('"')) {
            let Some((pattern, rest)) = line.split_once("=>") else { continue };
            let refuses = rest.trim_start().starts_with("Err(");
            for id in pattern.split('|').map(|p| p.trim().trim_matches('"')) {
                assert_eq!(syncs(id), !refuses, "`{id}`: syncs() and sync_inner disagree");
                arms += 1;
            }
        }
        assert!(arms >= BESPOKE_SYNC.len() + 2, "read {arms} arms — the reader has drifted");
        for id in BESPOKE_SYNC {
            assert!(body.contains(&format!("\"{id}\" =>")), "`{id}` has no arm in sync_inner");
        }
    }

    #[test]
    fn syncs_covers_the_generic_registries_and_nothing_else() {
        assert!(syncs("jira"), "a REST spec syncs");
        assert!(syncs("dropbox"), "an OAuth spec with feeds syncs");
        assert!(syncs("googlecalendar"), "the frontend's alias names a synced connector");
        assert!(!syncs("telegram"), "Telegram is reached by message, not by sync");
        assert!(!syncs("monday"), "catalogued, not built");
    }
