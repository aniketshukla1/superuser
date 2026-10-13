use super::*;
    use serde_json::json;

    #[test]
    fn dig_follows_dotted_paths() {
        let v = json!({"data": {"issues": {"nodes": [1, 2]}}, "name": "x"});
        assert_eq!(dig(&v, "name").as_str(), Some("x"));
        assert_eq!(dig(&v, "data.issues.nodes").as_array().unwrap().len(), 2);
        assert!(dig(&v, "nope.missing").is_null());
        assert!(dig(&v, "").is_object()); // empty path = root
    }

    #[test]
    fn creds_bare_and_map() {
        let bare = Creds::parse("tok_123");
        assert_eq!(bare.token(), "tok_123");
        assert_eq!(bare.site(), "");

        let map = Creds::parse(r#"{"email":"a@b.c","api_token":"t0k","site":"acme.atlassian.net"}"#);
        assert_eq!(map.token(), "t0k");
        assert_eq!(map.field("email").as_deref(), Some("a@b.c"));
        assert_eq!(map.site(), "acme.atlassian.net");
    }

    #[test]
    fn url_substitutes_site() {
        let spec = RestSpec { id: "jira", name: "Jira", base: "https://{site}/rest/api/3", auth: AuthStyle::BasicEmailToken, verify_path: "/myself", identity_at: "displayName", feeds: &[], extra_headers: &[] };
        let creds = Creds::parse(r#"{"site":"acme.atlassian.net","api_token":"t"}"#);
        assert_eq!(url_for(&spec, &creds, "/search"), "https://acme.atlassian.net/rest/api/3/search");
    }

    #[test]
    fn specs_are_well_formed() {
        assert!(SPECS.len() >= 17, "expected the ported REST registry, got {}", SPECS.len());
        let mut seen = std::collections::HashSet::new();
        for s in SPECS {
            assert!(seen.insert(s.id), "duplicate REST spec: {}", s.id);
            assert!(s.base.starts_with("https://"), "{} base must be https", s.id);
            assert!(s.verify_path.starts_with('/'), "{} verify_path must start with /", s.id);
            assert!(!s.name.is_empty());
            for f in s.feeds {
                assert!(f.path.starts_with('/'), "{} feed path must start with /", s.id);
                assert!(!f.id_key.is_empty() && !f.title_keys.is_empty(), "{} feed needs id/title keys", s.id);
                assert!(!f.kind.is_empty());
            }
            // A site-templated base must be paired with a site/subdomain field.
            if s.base.contains("{site}") {
                let c = super::super::descriptors::find(s.id).expect("spec must be in the catalog");
                assert!(c.fields.iter().any(|f| f.key == "site" || f.key == "subdomain"), "{} needs a site field", s.id);
            }
        }
    }

    #[test]
    fn text_of_handles_nested_shapes() {
        assert_eq!(text_of(&json!("hi")), "hi");
        assert_eq!(text_of(&json!(42)), "42");
        assert_eq!(text_of(&json!({"name": "Acme"})), "Acme");
        assert_eq!(text_of(&json!({"title": "T"})), "T");
        assert_eq!(text_of(&json!(["a", "b"])), "a, b");
        assert_eq!(text_of(&json!(null)), "");
    }
