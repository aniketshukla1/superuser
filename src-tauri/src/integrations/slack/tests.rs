use super::*;

    #[test]
    fn watermarks_roundtrip() {
        static SCRATCH_N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let seq = SCRATCH_N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let dir = std::env::temp_dir().join(format!("bba-slack-{}-{}-{}", std::process::id(), n, seq));
        std::fs::create_dir_all(&dir).unwrap();

        assert!(load_watermarks(&dir).is_empty());
        let mut m = serde_json::Map::new();
        m.insert("C123".into(), Value::String("1699999999.000100".into()));
        save_watermarks(&dir, &m);

        let back = load_watermarks(&dir);
        assert_eq!(back["C123"].as_str(), Some("1699999999.000100"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn slack_ts_ordering_is_numeric() {
        // Watermark comparison must be numeric, not lexicographic.
        let a = "1699999999.000100".parse::<f64>().unwrap();
        let b = "1700000000.000001".parse::<f64>().unwrap();
        assert!(b > a);
    }
