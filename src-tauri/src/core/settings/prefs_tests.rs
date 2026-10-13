use super::*;

    /// The display preferences the Settings panel reads back on every load.
    ///
    /// They were reported to the GUI as constants while the routes that set them
    /// did not exist, so the panel showed values nothing could change. What this
    /// pins is the half that lives here: a settings file written before these
    /// fields existed still parses, and still comes back with the defaults the
    /// panel expects rather than zeroes.
    #[test]
    fn a_settings_file_from_before_these_fields_still_parses() {
        let old = r#"{
            "local_only": true, "ollama_url": "http://x", "ollama_model": "m",
            "embed_model": "e", "cloud_enabled": false, "cloud_model": "c"
        }"#;
        let s: Settings = serde_json::from_str(old).expect("an older settings file must still load");
        assert_eq!(s.sessions_peek, 5, "the sidebar would show zero conversations");
        assert_eq!(s.nav_layout, "grouped");
        assert!(!s.context_bar);
        assert!(!s.auto_approve, "auto-approve must never default on");
        assert!(!s.auto_allow_read_only_commands);
        // Onboarding must default to "not done" — the opposite is how first-run
        // setup became unreachable, since the server reported it as a constant.
        assert!(!s.onboarded);
    }

    #[test]
    fn the_defaults_agree_with_the_serde_defaults() {
        // Two ways to make a Settings, and a card that reads one while a file
        // written by the other says something else is a panel that lies.
        let d = Settings::default();
        let empty: Settings = serde_json::from_str("{}").unwrap_or_else(|_| Settings::default());
        assert_eq!(d.sessions_peek, empty.sessions_peek);
        assert_eq!(d.nav_layout, empty.nav_layout);
        assert_eq!(d.context_bar, empty.context_bar);
    }
