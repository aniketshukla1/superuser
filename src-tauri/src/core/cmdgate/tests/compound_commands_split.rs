use super::*;

    pub(super) fn tok(s: &str) -> Vec<String> {
        tokenize(s).expect("parses")
    }

    #[test]
    pub(super) fn compound_commands_split_into_parts() {
        assert_eq!(split_commands("git status && git diff"), vec!["git status", "git diff"]);
        assert_eq!(split_commands("a | b ; c & d"), vec!["a", "b", "c", "d"]);
        // `&&` must not be read as two `&`, which would leave empty parts.
        assert_eq!(split_commands("x&&y").len(), 2);
    }

    #[test]
    pub(super) fn an_unterminated_quote_is_not_parsed_into_a_guess() {
        assert!(tokenize("echo \"hello").is_none());
        assert!(!auto_allowable("echo \"hello", &["echo".to_string()]));
    }

    #[test]
    pub(super) fn quotes_and_escapes_survive_into_argv() {
        assert_eq!(tok("git commit -m \"two words\""), vec!["git", "commit", "-m", "two words"]);
        assert_eq!(tok("ls My\\ Folder"), vec!["ls", "My Folder"]);
    }
