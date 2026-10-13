use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    fn scratch(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let d = std::env::temp_dir().join(format!("bba-dur-{tag}-{nanos}"));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    #[ignore]
    fn a_reader_never_catches_the_watch_store_mid_write() {
        use crate::core::subconscious::{Subconscious, Watch};
        let dir = scratch("watches");
        {
            let mut s = Subconscious::load(&dir);
            for i in 0..60 {
                s.create(Watch::new(format!("watch number {i} about something"), vec!["gmail".into()], false));
            }
        }

        let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let empties = Arc::new(AtomicUsize::new(0));
        let reads = Arc::new(AtomicUsize::new(0));

        let writer = {
            let dir = dir.clone();
            let stop = stop.clone();
            std::thread::spawn(move || {
                let mut n = 0;
                while !stop.load(Ordering::Relaxed) {
                    let mut s = Subconscious::load(&dir);
                    s.set_enabled(&s.watches[n % 60].id.clone(), n % 2 == 0);
                    n += 1;
                }
                n
            })
        };
        let mut readers = Vec::new();
        for _ in 0..4 {
            let dir = dir.clone();
            let stop = stop.clone();
            let empties = empties.clone();
            let reads = reads.clone();
            readers.push(std::thread::spawn(move || {
                while !stop.load(Ordering::Relaxed) {
                    let s = Subconscious::load(&dir);
                    reads.fetch_add(1, Ordering::Relaxed);
                    if s.watches.is_empty() {
                        empties.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }));
        }
        std::thread::sleep(std::time::Duration::from_millis(600));
        stop.store(true, Ordering::Relaxed);
        let writes = writer.join().unwrap();
        for r in readers {
            r.join().unwrap();
        }

        let (e, r) = (empties.load(Ordering::Relaxed), reads.load(Ordering::Relaxed));
        println!("{writes} writes against {r} reads → {e} reads saw an empty store");
        assert_eq!(e, 0, "{e} of {r} reads saw no watches at all while one was being saved");
        std::fs::remove_dir_all(&dir).ok();
    }
