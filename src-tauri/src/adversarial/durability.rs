use crate::core::store::write_atomic;

    #[test]
    fn an_atomic_write_is_durable_across_a_crash() {
        // rename() is atomic w.r.t. other readers, but without fsync of the file
        // before the rename and of the directory after, a power loss can leave a
        // zero-length or stale file. Probe: is the data flushed at all?
        let d = super::scratch("store-fsync");
        let p = d.join("thing.json");
        write_atomic(&p, "{\"a\":1}").unwrap();
        let src = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/core/store.rs"),
        )
        .unwrap();
        std::fs::remove_dir_all(&d).ok();
        assert!(
            src.contains("sync_all") || src.contains("sync_data"),
            "write_atomic never fsyncs — a crash between write() and rename() can leave the \
             store empty, which every reader turns into an empty store via unwrap_or_default()"
        );
    }

    #[test]
    fn concurrent_writers_do_not_leave_temp_files_behind() {
        let d = super::scratch("store-tmp");
        let p = d.join("s.json");
        let hs: Vec<_> = (0..8)
            .map(|i| {
                let p = p.clone();
                std::thread::spawn(move || {
                    for n in 0..200 {
                        write_atomic(&p, &format!("{{\"w\":{i},\"n\":{n}}}")).unwrap();
                    }
                })
            })
            .collect();
        for h in hs {
            h.join().unwrap();
        }
        let leftovers: Vec<String> = std::fs::read_dir(&d)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n != "s.json")
            .collect();
        std::fs::remove_dir_all(&d).ok();
        assert!(leftovers.is_empty(), "temp files survived concurrent writes: {leftovers:?}");
    }
