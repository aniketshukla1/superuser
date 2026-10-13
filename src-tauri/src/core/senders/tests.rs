use super::*;

    #[test]
    fn parses_targets() {
        assert_eq!(parse_target("slack:#ops"), ("slack".into(), "#ops".into()));
        assert_eq!(parse_target("telegram:12345"), ("telegram".into(), "12345".into()));
        assert_eq!(parse_target("SLACK:#Ops"), ("slack".into(), "#Ops".into()));
        // bare address → default channel
        assert_eq!(parse_target("12345"), ("telegram".into(), "12345".into()));
    }

    #[tokio::test]
    async fn empty_text_and_unknown_channel_are_refused() {
        assert!(send("telegram:1", "   ").await.is_err());
        let err = send("carrierpigeon:bob", "hi").await.unwrap_err();
        assert!(err.contains("isn't a sendable channel"), "got: {err}");
        // every advertised channel is actually routed (an unconnected one fails on
        // credentials, never on "unsupported")
        for ch in SENDABLE {
            let e = send(&format!("{ch}:x"), "hi").await.unwrap_err();
            assert!(!e.contains("isn't a sendable channel"), "{ch} should be routed, got: {e}");
        }
    }

    #[test]
    fn every_sendable_channel_is_one_the_access_panel_can_gate() {
        // The Read/Write gate keys on the platform `parse_target` returns, so each
        // entry must be exactly that platform string — a mismatch here would draw a
        // control that gates nothing.
        for ch in SENDABLE {
            assert!(is_sendable(ch), "{ch} should be sendable");
            assert_eq!(parse_target(&format!("{ch}:addr")).0, *ch);
        }
        assert!(is_sendable("SLACK"), "matching must be case-insensitive");
        assert!(!is_sendable("notion"), "a read-only source is not sendable");
    }
