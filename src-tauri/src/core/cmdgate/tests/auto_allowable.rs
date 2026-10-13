//! auto_allowable

use super::*;

    #[test]
    pub(super) fn a_blessed_prefix_runs_and_an_unblessed_one_does_not() {
        let rules = vec!["git status".to_string(), "cargo test".to_string()];
        assert!(auto_allowable("git status", &rules));
        assert!(auto_allowable("git status --short", &rules));
        assert!(auto_allowable("git status && cargo test", &rules));
        assert!(!auto_allowable("git push", &rules));
        // One unjustifiable part disqualifies the whole line.
        assert!(!auto_allowable("git status && git push", &rules));
    }

    #[test]
    pub(super) fn a_rule_matches_whole_tokens_only() {
        let rules = vec!["git status".to_string()];
        assert!(!auto_allowable("git statusfoo", &rules));
        assert!(!auto_allowable("git", &vec!["git status".to_string()]));
    }

    #[test]
    pub(super) fn substitution_redirection_and_expansion_all_disqualify() {
        let rules = vec!["echo".to_string(), "cat".to_string()];
        for cmd in ["echo `whoami`", "echo $(id)", "echo $HOME", "cat x > /etc/hosts", "cat < x"] {
            assert!(!auto_allowable(cmd, &rules), "should need approval: {cmd}");
        }
        assert!(auto_allowable("echo hello", &rules));
    }

    #[test]
    pub(super) fn no_rules_means_nothing_is_silent() {
        assert!(!auto_allowable("git status", &[]));
    }

    #[test]
    pub(super) fn a_hard_denied_command_is_never_auto_allowed_however_blessed() {
        assert!(!auto_allowable("rm -rf /", &["rm".to_string()]));
    }
