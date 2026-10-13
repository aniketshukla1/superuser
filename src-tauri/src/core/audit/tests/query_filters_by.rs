use super::*;

    #[test]
    pub(super) fn a_url_loses_its_query_its_fragment_and_its_password() {
        assert_eq!(clean_url("https://u:p@example.com/a/b?token=1#frag"), "https://example.com/a/b");
        assert_eq!(clean_url("https://example.com"), "https://example.com");
        assert_eq!(clean_url("https://example.com/?q=1"), "https://example.com/");
    }

    #[test]
    pub(super) fn the_initiator_is_read_off_the_platforms_own_session_names() {
        assert_eq!(Initiator::from_session("s-8f2a"), Initiator::new("person", ""));
        assert_eq!(Initiator::from_session("__run__run-6f9c"), Initiator::new("schedule", "run-6f9c"));
        assert_eq!(Initiator::from_session("watch-ab12"), Initiator::new("watch", "watch-ab12"));
        assert_eq!(Initiator::from_session("slack-C01-1717-1"), Initiator::new("message", "slack"));
        assert_eq!(Initiator::from_session("telegram-42"), Initiator::new("message", "telegram"));
        assert_eq!(Initiator::from_session("abc-sub2"), Initiator::new("subagent", "abc"));
        assert_eq!(Initiator::from_session("__run__x-sub1"), Initiator::new("subagent", "__run__x"));
        assert_eq!(Initiator::from_session("run-1717000"), Initiator::new("api", ""));
        // A name that merely contains "-sub" is not a subagent.
        assert_eq!(Initiator::from_session("my-subject"), Initiator::new("person", ""));
        assert!(Initiator::new("schedule", "").unwatched());
        assert!(!Initiator::new("subagent", "").unwatched());
    }

    #[test]
    pub(super) fn query_filters_by_session_family_and_refusal() {
        let dir = scratch();
        let mut a = Entry::event("action.allowed");
        a.session = "s1".into();
        a.initiator = Some(Initiator::new("schedule", "t1"));
        a.decision = Some(Decision { allowed: true, mode: "enforce".into(), source: "allow".into(), carried_out: true, ..Default::default() });
        append(&dir, a);
        let mut r = Entry::event("action.refused");
        r.session = "s2".into();
        r.initiator = Some(Initiator::new("person", ""));
        r.decision = Some(Decision { allowed: false, mode: "enforce".into(), source: "deny".into(), carried_out: false, ..Default::default() });
        append(&dir, r);
        log(&dir, "sync", "github", "issues", "2 new", true);

        assert_eq!(query(&dir, &Filter { session: Some("s1".into()), ..Default::default() }).len(), 1);
        assert_eq!(query(&dir, &Filter { event: Some("action".into()), ..Default::default() }).len(), 2);
        assert_eq!(query(&dir, &Filter { refused: true, ..Default::default() })[0].session, "s2");
        assert_eq!(query(&dir, &Filter { nobody_watching: true, ..Default::default() })[0].session, "s1");
        assert_eq!(query(&dir, &Filter { decisions: true, ..Default::default() }).len(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }
