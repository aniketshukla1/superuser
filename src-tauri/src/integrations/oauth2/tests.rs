use super::*;

    #[test]
    fn specs_are_well_formed() {
        assert!(SPECS.len() >= 8);
        let mut seen = std::collections::HashSet::new();
        for s in SPECS {
            assert!(seen.insert(s.id), "duplicate OAuth spec: {}", s.id);
            assert!(s.auth_url.starts_with("https://"), "{} auth_url", s.id);
            assert!(s.token_url.starts_with("https://"), "{} token_url", s.id);
            assert!(!s.scopes.is_empty(), "{} needs scopes", s.id);
            // every OAuth spec must exist in the connector catalog
            let c = super::super::descriptors::find(s.id).unwrap_or_else(|| panic!("{} missing from catalog", s.id));
            assert!(matches!(c.auth, super::super::descriptors::Auth::OAuth), "{} should be an OAuth connector", s.id);
            for f in s.feeds {
                assert!(f.path.starts_with('/'), "{} feed path", s.id);
            }
        }
    }

    #[test]
    fn post_feeds_carry_a_json_body() {
        let dropbox = find_spec("dropbox").unwrap();
        let f = dropbox.feeds.first().expect("dropbox should have a listing feed");
        let body = f.post_body.expect("dropbox listing is POST-only");
        assert!(serde_json::from_str::<Value>(body).is_ok(), "post body must be valid JSON: {body}");
        // GET feeds must not carry one.
        let drive = find_spec("google_drive").unwrap();
        assert!(drive.feeds[0].post_body.is_none());
    }

    #[test]
    fn urlencode_escapes_reserved_chars() {
        assert_eq!(urlencode("a b&c=d"), "a%20b%26c%3Dd");
        assert_eq!(urlencode("safe-_.~"), "safe-_.~");
        assert_eq!(urlencode("http://127.0.0.1:1234"), "http%3A%2F%2F127.0.0.1%3A1234");
    }

    #[test]
    fn param_reads_query_values() {
        assert_eq!(param("code=abc&state=xyz", "code").as_deref(), Some("abc"));
        assert_eq!(param("code=abc&state=xyz", "state").as_deref(), Some("xyz"));
        assert_eq!(param("error=denied", "code"), None);
    }
