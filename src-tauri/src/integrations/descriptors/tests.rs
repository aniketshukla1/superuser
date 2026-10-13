use super::*;
    use std::collections::HashSet;

    #[test]
    fn catalog_is_well_formed() {
        assert!(CONNECTORS.len() >= 38, "expected the full ported catalog, got {}", CONNECTORS.len());
        let mut ids = HashSet::new();
        for c in CONNECTORS {
            assert!(ids.insert(c.id), "duplicate connector id: {}", c.id);
            assert!(!c.title.is_empty() && !c.blurb.is_empty(), "{} missing title/blurb", c.id);
            assert!(c.brand_color.starts_with('#'), "{} brand color", c.id);
            assert!(!c.instructions.is_empty(), "{} has no setup instructions", c.id);
            // Anything needing a pasted credential must declare at least one field.
            if matches!(c.auth, Auth::ApiToken | Auth::Token | Auth::BotToken | Auth::AppPassword) {
                assert!(!c.fields.is_empty(), "{} needs input fields for its auth kind", c.id);
                assert!(c.fields.iter().any(|f| f.secret), "{} should have a secret field", c.id);
            }
        }
    }

    #[test]
    fn implemented_connectors_have_a_real_client() {
        // Honesty invariant: `implemented: true` must mean a client actually exists —
        // either a bespoke module wired into sync_one, or a generic REST spec.
        // Connectors with a bespoke sync client wired into sync_one.
        const BESPOKE: &[&str] = &["github", "notion", "gmail", "calendar", "linear", "stripe", "telegram", "slack", "email"];
        // Outbound-only: they can be sent to (core::senders) but not read from —
        // WhatsApp's Cloud API delivers inbound over webhooks, which we don't host.
        const SEND_ONLY: &[&str] = &["whatsapp"];
        // Queried live by an assistant's tools and never synced: metrics go stale in a minute.
        const LIVE: &[&str] = &["prometheus"];
        for c in CONNECTORS.iter().filter(|c| c.implemented) {
            let has_client = BESPOKE.contains(&c.id)
                || SEND_ONLY.contains(&c.id)
                || LIVE.contains(&c.id)
                || super::super::rest::find_spec(c.id).is_some()
                // An OAuth connector counts as implemented only once it can actually read.
                || super::super::oauth2::find_spec(c.id).map(|s| !s.feeds.is_empty()).unwrap_or(false);
            assert!(has_client, "{} is marked implemented but has no client", c.id);
        }
        // …and every REST spec should be surfaced as implemented in the catalog.
        for spec in super::super::rest::SPECS {
            let c = find(spec.id).unwrap_or_else(|| panic!("REST spec {} missing from the catalog", spec.id));
            assert!(c.implemented, "{} has a REST spec but isn't marked implemented", spec.id);
        }
    }

    #[test]
    fn lookup_and_aliases() {
        assert_eq!(find("slack").unwrap().title, "Slack");
        assert!(find("nope").is_none());
        let cal = find("calendar").unwrap();
        assert!(cal.aliases.contains(&"meetings"));
    }
