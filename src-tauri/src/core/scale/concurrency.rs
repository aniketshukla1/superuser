use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    fn scratch(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let d = std::env::temp_dir().join(format!("bba-conc-{tag}-{nanos}"));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// Every outbound request appends to one file, and the file rotates itself by
    /// rewriting in place. Rotation is a read-modify-write, so the question is what
    /// a concurrent append does to it — a torn line is a row the Privacy page
    /// silently drops, which is the one failure this log must not have.
    #[test]
    #[ignore]
    fn the_egress_log_survives_concurrent_writers() {
        let dir = scratch("egress");
        crate::core::egress::set_data_dir(&dir);
        let written = Arc::new(AtomicUsize::new(0));

        let mut handles = Vec::new();
        for t in 0..8 {
            let written = written.clone();
            handles.push(std::thread::spawn(move || {
                for i in 0..3_000 {
                    crate::core::egress::record_for_test(&format!("host{}.example.com", (t * 7 + i) % 30), "github", 128);
                    written.fetch_add(1, Ordering::Relaxed);
                }
            }));
        }
        for h in handles {
            h.join().unwrap();
        }

        let raw = std::fs::read_to_string(dir.join("egress.jsonl")).unwrap();
        let lines: Vec<&str> = raw.lines().filter(|l| !l.trim().is_empty()).collect();
        let bad: Vec<&&str> = lines
            .iter()
            .filter(|l| serde_json::from_str::<serde_json::Value>(l).is_err())
            .collect();
        println!(
            "24,000 concurrent records → {} retained lines, {} unparseable",
            lines.len(),
            bad.len()
        );
        if let Some(first) = bad.first() {
            println!("  first bad line: {:?}", &first.chars().take(120).collect::<String>());
        }
        assert!(bad.is_empty(), "{} torn lines — the Privacy page would drop them silently", bad.len());
        // And the readers must still work on whatever survived.
        assert!(!crate::core::egress::by_host(&dir).is_empty());
        std::fs::remove_dir_all(&dir).ok();
    }

    /// The profile freeze is a process-wide map keyed by session. Several
    /// conversations run at once (the user's turn, an automation, a watch
    /// escalation), and each must get its own frozen copy without blocking the
    /// others or handing one session another's block.
    #[test]
    #[ignore]
    fn each_session_gets_its_own_frozen_profile() {
        use crate::core::profile::{Class, Cue, Learned, Observation};
        let dir = scratch("profile");
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs_f64();
        let mut l = Learned::load(&dir);
        l.observe(vec![Observation {
            class: Class::Style,
            key: "reply-length".into(),
            value: "Wants short answers".into(),
            cue: Cue::Explicit,
            at: now,
            evidence: "said so".into(),
            source: "s-1".into(),
        }]);

        let mut handles = Vec::new();
        for t in 0..16 {
            let dir = dir.clone();
            handles.push(std::thread::spawn(move || {
                let mut seen = Vec::new();
                for i in 0..40 {
                    seen.push(crate::core::profile::frozen_block(&dir, &format!("s-{}", (t + i) % 8)));
                }
                seen
            }));
        }
        let mut blocks = Vec::new();
        for h in handles {
            blocks.extend(h.join().unwrap());
        }
        // Every session sees the same content here (one profile, no writes racing
        // it) — what matters is that none came back empty or garbled under load.
        assert!(blocks.iter().all(|b| b.contains("Wants short answers")), "a concurrent reader got a partial profile");
        println!("{} concurrent frozen_block reads across 8 sessions, all intact", blocks.len());
        crate::core::profile::thaw_all();
        std::fs::remove_dir_all(&dir).ok();
    }

    /// Local-only is read on every request and written when settings are saved.
    /// A flip must be seen by in-flight callers rather than only by new ones —
    /// otherwise turning it on leaves whatever is already running free to dial out.
    #[test]
    #[ignore]
    fn flipping_local_only_is_seen_by_everyone_at_once() {
        let readers = Arc::new(AtomicUsize::new(0));
        let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let mut handles = Vec::new();
        for _ in 0..8 {
            let readers = readers.clone();
            let stop = stop.clone();
            handles.push(std::thread::spawn(move || {
                let mut saw_on = false;
                while !stop.load(Ordering::Relaxed) {
                    if crate::core::egress::local_only() {
                        saw_on = true;
                    }
                    readers.fetch_add(1, Ordering::Relaxed);
                }
                saw_on
            }));
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
        crate::core::egress::set_local_only(true);
        std::thread::sleep(std::time::Duration::from_millis(50));
        stop.store(true, Ordering::Relaxed);

        let saw: Vec<bool> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        println!("{} policy reads across 8 threads; {} saw the flip", readers.load(Ordering::Relaxed), saw.iter().filter(|s| **s).count());
        assert!(saw.iter().all(|s| *s), "a thread never observed local-only being switched on");
        crate::core::egress::set_local_only(false);
    }
