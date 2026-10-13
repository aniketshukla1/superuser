use super::*;

    #[test]
    fn the_login_entry_points_at_where_the_app_actually_is() {
        // A build-time path would keep launching the old location after the user
        // moves the app, and they would only notice it had stopped.
        let entry = autostart_entry("/Applications/Superuser.app/Contents/MacOS/superuser");
        assert!(entry.contains("/Applications/Superuser.app/Contents/MacOS/superuser"));
        if cfg!(target_os = "macos") {
            assert!(entry.contains("RunAtLoad"));
            assert!(entry.starts_with("<?xml"));
        } else if cfg!(target_os = "linux") {
            assert!(entry.contains("[Desktop Entry]"));
        }
    }

    #[test]
    fn the_login_entry_lives_where_the_os_looks_for_it() {
        let p = autostart_path();
        if cfg!(target_os = "macos") {
            let p = p.expect("macOS has a LaunchAgents folder");
            assert!(p.to_string_lossy().contains("LaunchAgents"));
            assert!(p.extension().unwrap() == "plist");
        } else if cfg!(target_os = "linux") {
            assert!(p.unwrap().to_string_lossy().contains(".config/autostart"));
        } else {
            assert!(p.is_none(), "an unsupported platform should say so, not guess a path");
        }
    }

    #[test]
    fn keeping_awake_names_a_real_command_for_this_platform() {
        match keep_awake_command() {
            Some((program, args)) => {
                assert!(!program.is_empty() && !args.is_empty());
                if cfg!(target_os = "macos") {
                    assert_eq!(program, "caffeinate");
                    // Idle AND display: an agent working for ten minutes should not
                    // be interrupted by the screen dimming either.
                    assert!(args.contains(&"-di"));
                }
            }
            None => assert!(!cfg!(target_os = "macos") && !cfg!(target_os = "linux")),
        }
    }

    #[test]
    fn a_toggle_that_is_off_reports_off() {
        // And the guard survives being turned off when it was never on.
        assert_eq!(set_keep_awake(false).unwrap(), false);
        assert!(!keep_awake_enabled());
    }

    #[test]
    fn keeping_awake_round_trips() {
        if keep_awake_command().is_none() {
            return; // nothing to assert on this platform
        }
        assert!(set_keep_awake(true).unwrap(), "should report on");
        assert!(keep_awake_enabled(), "should still be holding sleep off");
        assert_eq!(set_keep_awake(false).unwrap(), false);
        assert!(!keep_awake_enabled(), "should have let go");
    }

    #[test]
    fn no_update_channel_says_so_rather_than_looking_like_up_to_date() {
        // The old behaviour returned null, which the banner read as "no update" —
        // indistinguishable from a working check that found nothing.
        let s = UpdateStatus::none("this build has no update channel configured");
        assert!(!s.available);
        assert!(s.reason.unwrap().contains("update channel"));
    }
