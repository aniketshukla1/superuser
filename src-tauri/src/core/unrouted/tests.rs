use super::*;

    fn scratch() -> PathBuf {
        use std::sync::atomic::{AtomicU32, Ordering};
        static N: AtomicU32 = AtomicU32::new(0);
        let p = std::env::temp_dir().join(format!("unrouted-{}-{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst)));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn a_dropped_message_is_recorded_with_why() {
        let d = scratch();
        let u = Unrouted::load(&d);
        assert!(u.list().is_empty());

        u.record("telegram", "5551234", "Ann (5551234)", "hello?", "this chat isn't on your allowlist");
        let items = u.list();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].source, "telegram");
        assert_eq!(items[0].chat_id, "5551234", "the allowlist needs the id, not the sentence");
        assert!(!items[0].id.is_empty(), "and the card needs a handle to act on");
        assert!(items[0].reason.contains("allowlist"), "the user has to know what to do about it");
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn the_newest_attempt_is_first() {
        let d = scratch();
        let u = Unrouted::load(&d);
        u.record("telegram", "1", "a", "first", "unlisted");
        u.record("telegram", "2", "b", "second", "unlisted");
        assert_eq!(u.list()[0].text, "second");
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn a_stranger_cannot_fill_the_disk() {
        // An unlisted chat can send as fast as it likes.
        let d = scratch();
        let u = Unrouted::load(&d);
        for i in 0..(KEEP + 40) {
            u.record("telegram", "9", "flooder", &format!("msg {i}"), "unlisted");
        }
        assert_eq!(u.list().len(), KEEP);
        assert_eq!(u.list()[0].text, format!("msg {}", KEEP + 39), "and the newest survive");
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn a_long_message_is_clamped() {
        let d = scratch();
        let u = Unrouted::load(&d);
        u.record("telegram", "x", "x", &"a".repeat(10_000), "unlisted");
        assert_eq!(u.list()[0].text.chars().count(), TEXT_CHARS);
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn recording_never_fails_the_caller() {
        // Message polling must not break because this could not be written.
        let u = Unrouted::load(Path::new("/proc/nonexistent-and-unwritable"));
        u.record("telegram", "x", "x", "y", "z"); // must not panic
        assert!(u.list().is_empty());
    }

    #[test]
    fn a_corrupt_file_reads_as_empty_rather_than_exploding() {
        let d = scratch();
        fs::write(d.join("unrouted.json"), "{ not json").unwrap();
        assert!(Unrouted::load(&d).list().is_empty());
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn a_drop_can_be_acted_on_exactly_once() {
        // The whole point of recording a drop is that the user can do something
        // about it. `take` is the only way the text leaves this store, and it has to
        // be one-shot: two clicks on "allow and deliver" must not answer twice.
        let d = scratch();
        let u = Unrouted::load(&d);
        u.record("telegram", "5551234", "Ann (5551234)", "can you look at this?", "unlisted");
        let id = u.list()[0].id.clone();

        let taken = u.take(&id).expect("the item is there");
        assert_eq!(taken.chat_id, "5551234");
        assert_eq!(taken.text, "can you look at this?");
        assert!(u.take(&id).is_none(), "a second click gets nothing");
        assert!(u.list().is_empty());
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn an_unknown_id_changes_nothing() {
        let d = scratch();
        let u = Unrouted::load(&d);
        u.record("telegram", "1", "a", "hello", "unlisted");
        assert!(!u.dismiss("no-such-id"));
        assert_eq!(u.list().len(), 1, "a bad id must not eat the queue");
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn drops_are_readable_per_connector() {
        let d = scratch();
        let u = Unrouted::load(&d);
        u.record("telegram", "1", "a", "hi", "unlisted");
        u.record("slack", "2", "b", "hi", "unlisted");
        assert_eq!(u.for_source("telegram").len(), 1);
        assert_eq!(u.for_source("slack").len(), 1);
        assert!(u.for_source("discord").is_empty());
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn a_record_written_before_ids_existed_is_still_actionable() {
        // The store predates `id`, so files on disk have none. Deriving one on read
        // keeps old drops clickable instead of quietly inert.
        let d = scratch();
        fs::write(
            d.join("unrouted.json"),
            r#"[{"source":"telegram","sender":"Ann (555)","text":"hello","reason":"unlisted","ts":1.0}]"#,
        )
        .unwrap();
        let u = Unrouted::load(&d);
        let items = u.list();
        assert_eq!(items.len(), 1);
        assert!(!items[0].id.is_empty(), "an old row still needs a handle");
        assert!(u.take(&items[0].id).is_some(), "and the handle has to work");
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn clearing_forgets_everything() {
        let d = scratch();
        let u = Unrouted::load(&d);
        u.record("telegram", "x", "x", "y", "z");
        u.clear();
        assert!(u.list().is_empty());
        let _ = fs::remove_dir_all(&d);
    }
