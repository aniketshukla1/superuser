use super::*;
    use crate::core::settings::Settings;

    fn settings(home: &str, allowed: &str) -> Settings {
        Settings { telegram_chat_id: home.into(), telegram_allowed: allowed.into(), ..Settings::default() }
    }

    #[test]
    fn only_configured_chats_are_served() {
        let s = settings("12345", "");
        assert!(is_allowed(&s, "12345"));
        assert!(!is_allowed(&s, "99999"), "a stranger who finds the bot gets nothing");
        assert!(!is_allowed(&s, ""));
    }

    #[test]
    fn an_unconfigured_bot_serves_nobody() {
        // The dangerous default would be "no allowlist means allow all" — a token
        // pasted into settings would then be an open door to the machine.
        let s = settings("", "");
        assert!(allowed_chats(&s).is_empty());
        assert!(!is_allowed(&s, "12345"));
        assert!(!is_allowed(&s, "anything"));
    }

    #[test]
    fn a_stranger_can_be_let_in_and_put_back_out() {
        // The Inbox shows who tried; this is what the "Allow" button does. Without
        // it the only route was hand-editing a comma-separated settings field.
        let mut s = settings("12345", "");
        assert!(allow(&mut s, "99999"));
        assert!(is_allowed(&s, "99999"));
        assert!(!allow(&mut s, "99999"), "adding twice is not a change");
        assert_eq!(s.telegram_allowed, "99999", "and does not duplicate the entry");

        assert_eq!(disallow(&mut s, "99999"), Ok(true));
        assert!(!is_allowed(&s, "99999"));
        assert_eq!(disallow(&mut s, "99999"), Ok(false), "removing what is gone is not a change");
    }

    #[test]
    fn the_home_chat_cannot_be_removed_from_here() {
        // It is where the app SENDS. Dropping it here would half-disconnect a
        // channel configured elsewhere, and the user would find out by not
        // receiving a briefing.
        let mut s = settings("12345", "99999");
        assert!(disallow(&mut s, "12345").is_err());
        assert!(is_allowed(&s, "12345"));
    }

    #[test]
    fn allowing_nothing_is_not_allowing_everyone() {
        let mut s = settings("", "");
        assert!(!allow(&mut s, "   "));
        assert!(allowed_chats(&s).is_empty(), "an empty id must not become an entry");
    }

    #[test]
    fn extra_chats_can_be_added_alongside_the_home_chat() {
        let s = settings("111", " 222 , 333 ");
        assert_eq!(allowed_chats(&s), vec!["111", "222", "333"]);
        assert!(is_allowed(&s, "222"));
        assert!(is_allowed(&s, "333"));
        assert!(!is_allowed(&s, "444"));
        // The home chat appearing in both fields shouldn't be listed twice.
        assert_eq!(allowed_chats(&settings("111", "111,222")), vec!["111", "222"]);
    }

    #[test]
    fn a_chat_maps_to_one_stable_session() {
        assert_eq!(session_id("12345"), "telegram-12345");
        assert_eq!(session_id("12345"), session_id("12345"), "the conversation continues");
        assert_ne!(session_id("1"), session_id("2"));
    }

    #[test]
    fn the_offset_round_trips() {
        let d = std::env::temp_dir().join(format!("bba-tg-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir_all(&d).unwrap();
        assert_eq!(load_offset(&d), 0, "a fresh install starts from the beginning");
        save_offset(&d, 42);
        assert_eq!(load_offset(&d), 42);
        std::fs::remove_dir_all(&d).ok();
    }
