use super::*;

    #[test]
    fn first_account_becomes_default() {
        let mut r = Registry::default();
        r.upsert("work@acme.com", "Work");
        r.upsert("me@home.com", "Personal");
        assert_eq!(r.accounts.len(), 2);
        assert!(r.accounts[0].default);
        assert!(!r.accounts[1].default);
        assert_eq!(r.default_id().as_deref(), Some("work@acme.com"));
    }

    #[test]
    fn upsert_relabels_without_duplicating() {
        let mut r = Registry::default();
        r.upsert("a", "First");
        r.upsert("a", "Renamed");
        assert_eq!(r.accounts.len(), 1);
        assert_eq!(r.accounts[0].label, "Renamed");
        assert!(r.accounts[0].default);
    }

    #[test]
    fn set_default_moves_the_flag() {
        let mut r = Registry::default();
        r.upsert("a", "A");
        r.upsert("b", "B");
        assert!(r.set_default("b"));
        assert_eq!(r.default_id().as_deref(), Some("b"));
        assert!(!r.accounts.iter().find(|x| x.id == "a").unwrap().default);
        assert!(!r.set_default("nope")); // unknown id is refused
    }

    #[test]
    fn removing_the_default_promotes_another() {
        let mut r = Registry::default();
        r.upsert("a", "A");
        r.upsert("b", "B");
        assert!(r.remove("a"));
        assert_eq!(r.accounts.len(), 1);
        assert_eq!(r.default_id().as_deref(), Some("b"));
        assert!(r.accounts[0].default);
        assert!(!r.remove("ghost"));
    }

    #[test]
    fn registry_roundtrips_through_json() {
        let mut r = Registry::default();
        r.upsert("a", "A");
        r.upsert("b", "B");
        r.set_default("b");
        let back = Registry::parse(Some(&r.to_json()));
        assert_eq!(back.accounts, r.accounts);
        // garbage / missing input yields an empty registry rather than an error
        assert!(Registry::parse(None).accounts.is_empty());
        assert!(Registry::parse(Some("not json")).accounts.is_empty());
    }
