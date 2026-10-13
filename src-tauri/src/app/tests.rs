use super::*;

    /// A scratch instance can hold the real credentials — launched with
    /// `SUPERUSER_SHARE_ACCOUNTS=1` (P1-3) — so the loops that reach the person's accounts on
    /// their own must not start in one. A source check,
    /// because the loops need a running app and the failure — replies in a real Slack — is
    /// the kind nobody notices until someone else does.
    #[test]
    fn a_scratch_instance_starts_nothing_that_reaches_real_accounts() {
        let src = crate::test_support::module_source("app");
        for f in ["start_scheduler", "start_slack_watcher", "start_telegram_watcher", "start_email_watcher"] {
            let body = &src[src.find(&format!("fn {f}(handle")).expect(f)..];
            let guard = body.find("core::instance::is_scratch()").unwrap_or(usize::MAX);
            let start = body.find("keep_running(").expect(f);
            assert!(guard < start, "{f} must check for a scratch instance before it starts");
        }
    }

    /// The data folder is decided once, in `resolve_data_dir`, and handed
    /// to whatever keeps files. A module that works it out for itself ignores
    /// `SUPERUSER_DATA_DIR`, and a scratch instance then writes into the real app's folder. The
    /// secrets store is the exception on purpose — model keys are shared — and `toolstats`
    /// resolves it only in a report that reads a real folder by design.
    #[test]
    fn nothing_works_out_the_data_folder_for_itself() {
        fn walk(dir: &std::path::Path, out: &mut Vec<PathBuf>) {
            for entry in std::fs::read_dir(dir).unwrap().flatten() {
                let p = entry.path();
                if p.is_dir() {
                    walk(&p, out);
                } else if p.extension().is_some_and(|e| e == "rs") {
                    out.push(p);
                }
            }
        }
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut files = Vec::new();
        walk(&src, &mut files);
        assert!(files.len() > 50, "found only {} source files", files.len());
        let needle = ["dirs::", "data_dir()"].concat();
        for f in files {
            let rel = f.strip_prefix(&src).unwrap().to_string_lossy().replace('\\', "/");
            // each exempt module, and the folder it was split into
            let exempt = |m: &str| rel == m || rel.starts_with(&format!("{}/", m.trim_end_matches(".rs")));
            if ["lib.rs", "app.rs", "core/secrets.rs", "core/toolstats.rs"].into_iter().any(exempt) {
                continue;
            }
            let text = std::fs::read_to_string(&f).unwrap();
            assert!(!text.contains(&needle), "{rel} works out the data folder for itself; take it from the caller");
        }
        // And the turn keeps what it cuts in the folder `recall_output` reads: its tools' own.
        let commands = crate::test_support::module_source("commands");
        let stores = commands.matches("ObservationStore::for_session(").count();
        assert!(stores >= 2, "the turn's observation stores have moved: found {stores}");
        assert_eq!(commands.matches("ObservationStore::for_session(&state.data_dir,").count(), stores);
    }

    #[test]
    fn the_nightly_wait_is_always_within_a_day_and_never_a_busy_loop() {
        let s = seconds_until_nightly();
        assert!(s >= 60, "never returns 0 — a zero wait would spin the scheduler");
        assert!(s <= 24 * 3600, "never more than a day away: got {s}s");
    }

    #[test]
    fn the_nightly_wait_lands_on_23_55_local() {
        use chrono::{Local, Timelike};
        let target = Local::now() + chrono::Duration::seconds(seconds_until_nightly() as i64);
        // Allow a minute of slack for the clock advancing between the two calls.
        let minutes_off = (target.hour() as i64 * 60 + target.minute() as i64 - (23 * 60 + 55)).abs();
        assert!(minutes_off <= 1, "woke at {}:{:02}, expected 23:55", target.hour(), target.minute());
    }
