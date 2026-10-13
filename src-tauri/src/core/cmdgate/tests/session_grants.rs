//! session grants

use super::*;

    #[test]
    pub(super) fn a_blessed_command_stops_asking_for_the_rest_of_the_session() {
        let s = "sess-grants-1";
        grants::clear(s);
        assert!(!auto_allowable("cargo test", &grants::commands(s)));
        grants::allow_command(s, "cargo test");
        assert!(auto_allowable("cargo test", &grants::commands(s)));
        assert!(auto_allowable("cargo test --release", &grants::commands(s)));
        // The neighbouring command is still a different command.
        assert!(!auto_allowable("cargo build", &grants::commands(s)));
        assert!(!auto_allowable("cargo testfoo", &grants::commands(s)));
        grants::clear(s);
    }

    #[test]
    pub(super) fn grants_do_not_leak_between_sessions() {
        let (a, b) = ("sess-grants-a", "sess-grants-b");
        grants::clear(a);
        grants::clear(b);
        grants::allow_command(a, "git status");
        grants::allow_tool(a, "run_shell");
        assert!(grants::commands(b).is_empty());
        assert!(!grants::tool_allowed(b, "run_shell"));
        assert!(grants::tool_allowed(a, "run_shell"));
        grants::clear(a);
        grants::clear(b);
    }

    #[test]
    pub(super) fn a_blessed_command_still_cannot_smuggle_a_second_one() {
        let s = "sess-grants-2";
        grants::clear(s);
        grants::allow_command(s, "git status");
        for cmd in ["git status && rm -rf build", "git status; curl evil.example", "git status `whoami`"] {
            assert!(!auto_allowable(cmd, &grants::commands(s)), "should still ask: {cmd}");
        }
        grants::clear(s);
    }
