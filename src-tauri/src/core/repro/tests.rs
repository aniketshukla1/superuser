use super::*;

    #[test]
    fn only_the_staging_site_is_reachable() {
        let staging = "https://staging.example.com";
        assert_eq!(on_staging("https://staging.example.com/cart?id=4", staging), Ok(()));
        assert_eq!(on_staging("staging.example.com/login", staging), Ok(()), "a bare address is read as https");
        assert!(on_staging("https://app.example.com/cart", staging).unwrap_err().contains("never production"));
        assert!(on_staging("https://staging.example.com.evil.io/", staging).is_err(), "a host is the whole host");
        assert!(on_staging("https://example.com/", staging).is_err());
    }

    #[test]
    fn the_port_is_part_of_the_site() {
        let staging = "http://localhost:3000";
        assert_eq!(on_staging("http://localhost:3000/checkout", staging), Ok(()));
        assert!(on_staging("http://localhost:8080/admin", staging).is_err(), "another service on the same machine is not staging");
    }

    #[test]
    fn without_a_staging_address_nothing_is_reachable() {
        assert!(on_staging("https://staging.example.com", "").unwrap_err().contains("Settings ▸ General ▸ Bug Reproduction"));
        assert!(on_staging("https://staging.example.com", "   ").is_err());
    }

    #[test]
    fn reading_a_page_off_staging_is_refused_as_much_as_acting_on_it() {
        let staging = "https://staging.example.com";
        for tool in ["browser_snapshot", "browser_read", "browser_console", "browser_screenshot", "browser_click", "browser_type"] {
            assert!(may_use(tool, Some("https://app.example.com/admin"), staging).is_err(), "{tool} on production");
            assert_eq!(may_use(tool, Some("https://staging.example.com/cart"), staging), Ok(()), "{tool} on staging");
            assert_eq!(may_use(tool, None, staging), Ok(()), "{tool} with nothing open has nothing to show");
            assert_eq!(may_use(tool, Some("about:blank"), staging), Ok(()));
        }
        assert!(may_use("browser_navigate", Some("https://app.example.com"), staging).is_err());
        assert!(may_use("browser_navigate", None, staging).is_err(), "opening nowhere isn't staging");
    }

    #[test]
    fn a_person_can_sign_in_on_another_site_for_it() {
        let staging = "https://staging.example.com";
        assert_eq!(may_use("browser_request_help", Some("https://login.example-sso.com/authorize"), staging), Ok(()));
        assert!(may_use("browser_request_help", None, "").is_err(), "without staging there's nothing to help with");
        assert!(may_use("browser_snapshot", None, "").unwrap_err().contains("no staging address"));
    }

    #[test]
    fn a_redirect_off_staging_is_named_and_kept_from_the_assistant() {
        let err = landed("https://app.example.com/dashboard?acct=77", "https://staging.example.com").unwrap_err();
        assert!(err.contains("`app.example.com`") && err.contains("browser_request_help"), "{err}");
        assert!(!err.contains("acct=77"), "only the site is named, not the page: {err}");
        assert_eq!(landed("https://staging.example.com/home", "https://staging.example.com"), Ok(()));
    }

    #[test]
    fn the_settings_field_takes_a_web_address() {
        assert_eq!(staging_problem("https://staging.example.com"), None);
        assert_eq!(staging_problem("staging.example.com"), None);
        assert_eq!(staging_problem(""), None, "empty is allowed — it means none");
        assert!(staging_problem("ftp://files.example.com").is_some());
        // A company's staging, and a dev server on this Mac, are both staging.
        assert_eq!(staging_problem("http://10.20.0.15:8080"), None);
        assert_eq!(staging_problem("https://staging.corp.internal"), None);
        assert_eq!(staging_problem("http://localhost:3000"), None);
        // What the browser never reaches is said at once.
        for never in ["http://169.254.169.254", "http://metadata.google.internal", "http://[fe80::1]:8080", "http://0.0.0.0"] {
            assert!(staging_problem(never).unwrap().contains("never reaches"), "{never}");
        }
    }

    #[test]
    fn the_private_exception_is_the_exact_staging_site() {
        let staging = "http://staging.corp.internal:8080";
        assert!(is_staging_site(staging, "staging.corp.internal", 8080));
        assert!(is_staging_site(staging, "Staging.Corp.Internal.", 8080), "the proxy's spelling of the same host");
        assert!(!is_staging_site(staging, "staging.corp.internal", 443), "another port is another service");
        assert!(!is_staging_site(staging, "db.corp.internal", 8080));
        assert!(is_staging_site("https://10.20.0.15", "10.20.0.15", 443), "the scheme's own port");
        assert!(is_staging_site("staging.example.com", "staging.example.com", 443), "a bare address is https");
        assert!(is_staging_site("http://[fd00::15]:3000", "fd00::15", 3000));
        assert!(!is_staging_site("", "", 0));
        assert!(!is_private_staging("assistant", "staging.corp.internal", 8080), "only Bug Reproduction's browser");
    }
