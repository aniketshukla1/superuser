use super::*;

    /// The profile is rebuilt from every observation ever recorded, on every read.
    /// `OBSERVATION_LIMIT` is 2,000, so this is the ceiling by construction — the
    /// question is whether the ceiling is comfortable.
    #[test]
    #[ignore]
    pub(super) fn profile_scoring_at_the_observation_cap() {
        use crate::core::profile::{Class, Cue, Learned, Observation};
        let dir = scratch("profile");
        let mut l = Learned::load(&dir);
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs_f64();

        let t = Instant::now();
        for i in 0..2_500 {
            l.observe(vec![Observation {
                class: Class::Style,
                key: format!("k{}", i % 200),
                value: format!("Prefers thing {}", i % 400),
                cue: Cue::Behavioral,
                at: now - (i as f64 * 3_600.0),
                evidence: "said so".into(),
                source: "s-1".into(),
            }]);
        }
        println!("2,500 observations written (each one persists the file): {:>8.1?}", t.elapsed());

        let t = Instant::now();
        let facets = l.facets();
        let scored = t.elapsed();
        println!("scoring {} facets from a capped log: {scored:>8.1?}", facets.len());
        assert!(scored.as_millis() < 2_000, "profile scoring took {scored:?}");

        // The read that happens on every new conversation.
        let t = Instant::now();
        let block = crate::core::profile::frozen_block(&dir, "s-scale");
        println!("frozen_block (first call, cold): {:>8.1?}, {} chars", t.elapsed(), block.len());
        assert!(block.len() < 8_000, "the profile block reached {} chars of every prompt", block.len());
        crate::core::profile::thaw_all();
        std::fs::remove_dir_all(&dir).ok();
    }

    /// The egress log is written on every outbound request and rotated in place.
    /// Rotation is the interesting part: it rewrites the file, and it happens while
    /// other requests are trying to append.
    #[test]
    #[ignore]
    pub(super) fn egress_log_under_sustained_traffic() {
        let dir = scratch("egress");
        crate::core::egress::set_data_dir(&dir);

        let t = Instant::now();
        for i in 0..20_000 {
            crate::core::egress::record_for_test(&format!("host{}.example.com", i % 40), "github", 512);
        }
        println!("20,000 recorded requests (with rotation): {:>8.1?}", t.elapsed());

        let t = Instant::now();
        let hosts = crate::core::egress::by_host(&dir);
        println!("rollup over the retained log: {:>8.1?} ({} hosts)", t.elapsed(), hosts.len());

        let t = Instant::now();
        let recent = crate::core::egress::recent(&dir, 300);
        println!("recent(300): {:>8.1?} ({} rows)", t.elapsed(), recent.len());
        assert_eq!(recent.len(), 300);
        // Rotation must not leave a half-written line behind: every retained line
        // has to still parse, or the page silently drops rows.
        let raw = std::fs::read_to_string(dir.join("egress.jsonl")).unwrap();
        let bad = raw.lines().filter(|l| !l.trim().is_empty() && serde_json::from_str::<serde_json::Value>(l).is_err()).count();
        assert_eq!(bad, 0, "{bad} unparseable lines survived rotation");
        std::fs::remove_dir_all(&dir).ok();
    }

    /// The run journal is written through every agent frame and read by the rail.
    #[test]
    #[ignore]
    pub(super) fn run_journal_at_length() {
        let dir = scratch("runlog");
        let t = Instant::now();
        for i in 0..20_000 {
            crate::core::runlog::append(&dir, &serde_json::json!({ "type": "tool_done", "data": { "name": "read_file", "seq": i } }));
        }
        println!("20,000 journal frames: {:>8.1?}", t.elapsed());

        let t = Instant::now();
        let page = crate::core::runlog::read(&dir, 0, 500);
        println!("read(0, 500): {:>8.1?} ({} events, truncated={})", t.elapsed(), page.events.len(), page.truncated);

        // The rail asks for the whole thing to show the tail; measure the worst case.
        let t = Instant::now();
        let all = crate::core::runlog::read(&dir, 0, 1_000);
        println!("read(0, 1000): {:>8.1?} ({} events)", t.elapsed(), all.events.len());
        std::fs::remove_dir_all(&dir).ok();
    }
