//! hard_deny

use super::*;

    #[test]
    pub(super) fn the_catastrophes_are_refused() {
        for cmd in [
            "sudo rm -rf /",
            "rm -rf /",
            "rm -rf ~",
            "rm -rf /usr",
            "rm -rf /Users/aniket",
            "rm -rf --no-preserve-root /",
            "mkfs.ext4 /dev/disk2",
            "dd if=/dev/zero of=/dev/disk2",
            "shutdown -h now",
            ":(){ :|:& };:",
            "echo x > /dev/sda",
        ] {
            assert!(hard_deny(cmd).is_some(), "should have been refused: {cmd}");
        }
    }

    #[test]
    pub(super) fn a_tab_after_sudo_no_longer_walks_through() {
        // The old rule looked for the literal `"sudo "`. This one looks at argv.
        assert!(hard_deny("sudo\trm -rf /").is_some());
        assert!(hard_deny("/usr/bin/sudo reboot").is_some());
    }

    #[test]
    pub(super) fn the_word_in_a_quoted_string_is_not_the_command() {
        // Every one of these was blocked by the substring denylist.
        for cmd in [
            "grep reboot /var/log/syslog",
            "echo \"sudo is not allowed here\"",
            "git commit -m \"drop the mkfs helper\"",
        ] {
            assert_eq!(hard_deny(cmd), None, "should have been allowed: {cmd}");
        }
    }

    #[test]
    pub(super) fn ordinary_recursive_deletes_still_work() {
        // `contains("rm -rf /")` matched every absolute path, so this was blocked.
        for cmd in ["rm -rf /tmp/build", "rm -rf ~/proj/dist", "rm -rf node_modules"] {
            assert_eq!(hard_deny(cmd), None, "should have been allowed: {cmd}");
        }
    }

    #[test]
    pub(super) fn a_catastrophe_hidden_in_the_second_half_is_still_found() {
        assert!(hard_deny("git status && sudo rm -rf /").is_some());
    }
