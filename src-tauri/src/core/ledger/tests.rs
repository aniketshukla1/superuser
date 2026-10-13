use super::*;

    fn scratch() -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(0);
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let p = std::env::temp_dir().join(format!("su-ledger-{}-{}", nanos, N.fetch_add(1, Ordering::SeqCst)));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn write(dir: &Path, name: &str, body: &str) {
        std::fs::write(dir.join(name), body).unwrap();
    }

    #[test]
    fn the_first_read_of_a_source_reports_everything_and_the_second_reports_nothing() {
        let v = scratch();
        let l = Ledger::new(&v);
        write(&v, "gmail-1.md", "an email");
        write(&v, "gmail-2.md", "another email");
        assert!(l.snapshot("gmail", "sync").unwrap().is_some());

        let first = l.changes_since_read("gmail", true).unwrap();
        assert_eq!(first.added.len(), 2, "nothing read yet, so all of it is new");
        assert!(first.acknowledged);

        // The point of the marker: polling again must not re-report the same mail.
        let second = l.changes_since_read("gmail", true).unwrap();
        assert!(second.is_empty(), "got {second:?}");
        assert_eq!(second.total, 2, "still two files, none of them new");
        std::fs::remove_dir_all(&v).ok();
    }

    #[test]
    fn a_source_only_sees_its_own_files() {
        let v = scratch();
        let l = Ledger::new(&v);
        write(&v, "gmail-1.md", "an email");
        write(&v, "github-1.md", "an issue");
        l.snapshot("initial", "sync").unwrap();

        let gmail = l.changes_since_read("gmail", true).unwrap();
        assert_eq!(gmail.added, vec!["gmail-1.md".to_string()]);
        // Reading Gmail must not consume GitHub's delta.
        let github = l.changes_since_read("github", true).unwrap();
        assert_eq!(github.added, vec!["github-1.md".to_string()]);
        std::fs::remove_dir_all(&v).ok();
    }

    #[test]
    fn a_preview_does_not_consume_the_delta() {
        let v = scratch();
        let l = Ledger::new(&v);
        write(&v, "slack-1.md", "a message");
        l.snapshot("slack", "sync").unwrap();

        let peek = l.changes_since_read("slack", false).unwrap();
        assert_eq!(peek.added.len(), 1);
        assert!(!peek.acknowledged);
        let again = l.changes_since_read("slack", true).unwrap();
        assert_eq!(again.added.len(), 1, "a look is not an acknowledgement");
        std::fs::remove_dir_all(&v).ok();
    }

    #[test]
    fn edits_and_deletions_are_told_apart_from_new_items() {
        let v = scratch();
        let l = Ledger::new(&v);
        write(&v, "notion-a.md", "first");
        write(&v, "notion-b.md", "second");
        l.snapshot("notion", "sync").unwrap();
        l.changes_since_read("notion", true).unwrap();

        write(&v, "notion-a.md", "first, edited");
        std::fs::remove_file(v.join("notion-b.md")).unwrap();
        write(&v, "notion-c.md", "third");
        l.snapshot("notion", "sync").unwrap();

        let d = l.changes_since_read("notion", true).unwrap();
        assert_eq!(d.added, vec!["notion-c.md".to_string()]);
        assert_eq!(d.modified, vec!["notion-a.md".to_string()]);
        assert_eq!(d.removed, vec!["notion-b.md".to_string()]);
        assert_eq!(d.summary(), "notion: 1 new, 1 changed, 1 removed.");
        std::fs::remove_dir_all(&v).ok();
    }

    #[test]
    fn a_sync_that_changed_nothing_leaves_no_snapshot() {
        let v = scratch();
        let l = Ledger::new(&v);
        write(&v, "stripe-1.md", "a charge");
        assert!(l.snapshot("stripe", "sync").unwrap().is_some());
        assert!(l.snapshot("stripe", "sync").unwrap().is_none(), "no empty commits");
        assert_eq!(l.snapshot_count(), 1);
        std::fs::remove_dir_all(&v).ok();
    }

    #[test]
    fn reading_a_source_before_anything_is_synced_is_an_error_not_a_lie() {
        // An empty answer here would read as "nothing new", which is a different
        // claim from "there is no history to compare against".
        let v = scratch();
        let l = Ledger::new(&v);
        assert!(l.changes_since_read("gmail", true).is_err());
        std::fs::remove_dir_all(&v).ok();
    }

    #[test]
    fn an_existing_repository_is_adopted_rather_than_re_initialized() {
        let v = scratch();
        let l = Ledger::new(&v);
        // A vault the user already keeps in git — its history must survive.
        Command::new("git").arg("-C").arg(&v).args(["init", "--quiet"]).output().unwrap();
        std::fs::write(v.join(".gitignore"), "mine\n").unwrap();
        write(&v, "gmail-1.md", "an email");
        l.snapshot("gmail", "sync").unwrap();
        assert_eq!(std::fs::read_to_string(v.join(".gitignore")).unwrap(), "mine\n");
        std::fs::remove_dir_all(&v).ok();
    }

    #[test]
    fn a_peek_then_acknowledge_consumes_exactly_once() {
        // The tick's shape: look, do the work, and only then claim to have seen it.
        let v = scratch();
        let l = Ledger::new(&v);
        write(&v, "gmail-1.md", "an email");
        l.snapshot("gmail", "sync").unwrap();

        assert_eq!(l.changes_since_read("gmail", false).unwrap().added.len(), 1);
        // Work failed here → no acknowledge → the delta is still waiting.
        assert_eq!(l.changes_since_read("gmail", false).unwrap().added.len(), 1);

        l.acknowledge("gmail").unwrap();
        assert!(l.changes_since_read("gmail", false).unwrap().is_empty());
        std::fs::remove_dir_all(&v).ok();
    }

    #[test]
    fn markers_are_listed_per_source() {
        let v = scratch();
        let l = Ledger::new(&v);
        write(&v, "gmail-1.md", "an email");
        write(&v, "slack-1.md", "a message");
        l.snapshot("all", "sync").unwrap();
        l.changes_since_read("gmail", true).unwrap();
        let names: Vec<String> = l.read_markers().into_iter().map(|(n, _)| n).collect();
        assert_eq!(names, vec!["gmail".to_string()], "only what has actually been read");
        std::fs::remove_dir_all(&v).ok();
    }
