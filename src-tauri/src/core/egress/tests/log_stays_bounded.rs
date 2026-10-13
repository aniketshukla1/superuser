use super::*;

    #[test]
    pub(super) fn the_log_stays_bounded() {
        let _lock = exclusive();
        let d = scratch();
        set_data_dir(&d);
        let line = Egress {
            at: now(),
            purpose: "x".repeat(200),
            method: "GET".into(),
            host: "example.com".into(),
            path: "/".into(),
            sent: 0,
            status: Some(200),
            error: None,
            blocked: false,
        };
        for _ in 0..3_000 {
            record(line.clone());
        }
        let size = std::fs::metadata(path_of(&d)).unwrap().len();
        assert!(size <= MAX_BYTES * 2, "log grew to {size} bytes");
        assert!(!recent(&d, 10).is_empty(), "trimming must not empty it");
                std::fs::remove_dir_all(&d).ok();
    }
