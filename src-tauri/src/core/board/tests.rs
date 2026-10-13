use super::*;

    fn scratch() -> PathBuf {
        use std::sync::atomic::{AtomicU32, Ordering};
        static N: AtomicU32 = AtomicU32::new(0);
        let p = std::env::temp_dir().join(format!("board-{}-{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst)));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    fn board(dir: &Path) -> Board {
        Board::open(dir, Path::new("/repo/thing"))
    }

    #[test]
    fn an_item_moves_through_the_columns() {
        let d = scratch();
        let b = board(&d);
        let id = b.add("lead", "Add the retry", "", "the flaky test passes ten times").unwrap();
        assert_eq!(b.get(&id).unwrap().state, ItemState::Open);
        b.assign("lead", &id, "worker").unwrap();
        b.move_to("worker", &id, ItemState::InProgress).unwrap();
        b.move_to("worker", &id, ItemState::Review).unwrap();
        let item = b.get(&id).unwrap();
        assert_eq!(item.state, ItemState::Review);
        assert_eq!(item.assignee.as_deref(), Some("worker"));
        assert_eq!(item.builder.as_deref(), Some("worker"), "the builder is whoever put it up");
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn the_builder_cannot_pass_its_own_work() {
        // The rule the module exists for. A model marking its own work complete is
        // not a rare failure; it is the default behaviour of anything optimising
        // to finish.
        let d = scratch();
        let b = board(&d);
        let id = b.add("lead", "Add the retry", "", "tests pass").unwrap();
        b.move_to("worker", &id, ItemState::Review).unwrap();

        let err = b.review("worker", &id, true, "I ran the tests").unwrap_err();
        assert!(err.contains("cannot be the one to pass it"), "got: {err}");
        assert!(b.get(&id).unwrap().reviews.is_empty(), "the self-review must not be recorded");
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn done_needs_a_passing_review_from_someone_else() {
        let d = scratch();
        let b = board(&d);
        let id = b.add("lead", "Add the retry", "", "tests pass").unwrap();
        b.move_to("worker", &id, ItemState::Review).unwrap();

        let err = b.move_to("worker", &id, ItemState::Done).unwrap_err();
        assert!(err.contains("no passing review"), "got: {err}");
        assert_eq!(b.get(&id).unwrap().state, ItemState::Review, "and it did not move");

        b.review("verifier", &id, true, "ran the suite ten times, green").unwrap();
        b.move_to("lead", &id, ItemState::Done).unwrap();
        assert_eq!(b.get(&id).unwrap().state, ItemState::Done);
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn a_failing_review_does_not_unlock_done() {
        let d = scratch();
        let b = board(&d);
        let id = b.add("lead", "Add the retry", "", "tests pass").unwrap();
        b.move_to("worker", &id, ItemState::Review).unwrap();
        b.review("verifier", &id, false, "still flaky on the third run").unwrap();
        assert!(b.move_to("lead", &id, ItemState::Done).is_err());
        assert!(!b.get(&id).unwrap().verified());
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn a_review_without_evidence_is_not_a_review() {
        let d = scratch();
        let b = board(&d);
        let id = b.add("lead", "x", "", "y").unwrap();
        b.move_to("worker", &id, ItemState::Review).unwrap();
        assert!(b.review("verifier", &id, true, "   ").is_err());
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn the_board_survives_a_corrupt_line() {
        // One bad append must not take the project's board with it.
        let d = scratch();
        let b = board(&d);
        let id = b.add("lead", "Keep me", "", "").unwrap();
        let mut f = OpenOptions::new().append(true).open(&b.path).unwrap();
        writeln!(f, "{{ not json at all").unwrap();
        drop(f);
        b.note("lead", &id, "still working").unwrap();

        let items = b.items();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].notes, vec!["still working".to_string()]);
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn events_about_an_unknown_item_are_skipped_not_fatal() {
        let d = scratch();
        let b = board(&d);
        b.append(&Event {
            at: now(),
            item: "ghost".into(),
            actor: "lead".into(),
            kind: EventKind::Noted { text: "about nothing".into() },
        })
        .unwrap();
        assert!(b.items().is_empty());
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn items_written_in_the_same_breath_are_different_items() {
        // Milliseconds alone collided, and the second Created replaced the first in
        // the projection — work disappearing off the board while someone filled it.
        let d = scratch();
        let b = board(&d);
        let ids: Vec<String> = (0..25).map(|i| b.add("lead", &format!("item {i}"), "", "").unwrap()).collect();
        let unique: std::collections::BTreeSet<&String> = ids.iter().collect();
        assert_eq!(unique.len(), ids.len(), "two items share an id");
        assert_eq!(b.items().len(), 25, "every one of them is on the board");
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn two_projects_do_not_share_a_board() {
        let d = scratch();
        let a = Board::open(&d, Path::new("/repo/one"));
        let c = Board::open(&d, Path::new("/repo/two"));
        a.add("lead", "only in one", "", "").unwrap();
        assert_eq!(a.items().len(), 1);
        assert!(c.items().is_empty());
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn the_rendering_leads_with_what_is_waiting_on_a_human_decision() {
        let d = scratch();
        let b = board(&d);
        let open = b.add("lead", "Not started", "", "").unwrap();
        let reviewing = b.add("lead", "Waiting on a check", "", "the suite is green").unwrap();
        b.move_to("worker", &reviewing, ItemState::Review).unwrap();
        let _ = open;

        let text = b.render(None);
        assert!(text.find("review").unwrap() < text.find("open").unwrap(), "review comes first:\n{text}");
        assert!(text.contains("needs a different reviewer"), "{text}");
        assert!(text.contains("done when: the suite is green"), "{text}");
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn an_empty_board_says_so_rather_than_returning_nothing() {
        let d = scratch();
        assert!(board(&d).render(None).contains("empty"));
        assert!(board(&d).render(Some(ItemState::Done)).contains("Nothing on the board is done"));
        let _ = fs::remove_dir_all(&d);
    }
