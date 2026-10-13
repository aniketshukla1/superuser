use super::*;

    #[test]
    fn every_connection_is_forced_through_the_proxy_and_the_protocol_has_no_port() {
        let a = args(Path::new("/tmp/p"), 51234, true);
        assert!(a.contains(&"--proxy-server=http://127.0.0.1:51234".to_string()));
        assert!(a.contains(&"--proxy-bypass-list=<-loopback>".to_string()), "loopback goes through the proxy too");
        assert!(a.contains(&"--disable-quic".to_string()));
        assert!(a.contains(&"--remote-debugging-pipe".to_string()));
        assert!(!a.iter().any(|x| x.starts_with("--remote-debugging-port")), "no port anything else could reach");
        assert!(!a.iter().any(|x| x == "--enable-automation"));
        assert_eq!(a.last().map(String::as_str), Some("about:blank"));
        assert!(a.contains(&"--headless=new".to_string()));
        assert!(!args(Path::new("/tmp/p"), 1, false).contains(&"--headless=new".to_string()));
    }

    #[cfg(unix)]
    #[test]
    fn a_lock_left_by_a_dead_process_is_cleared_and_a_live_stranger_is_left_alone() {
        let dir = std::env::temp_dir().join(format!("su-chrome-lock-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        // A pid that cannot exist.
        std::os::unix::fs::symlink("host-999999", dir.join("SingletonLock")).unwrap();
        std::fs::write(dir.join("SingletonCookie"), "x").unwrap();
        clear_stale_locks(&dir);
        assert!(std::fs::symlink_metadata(dir.join("SingletonLock")).is_err());
        assert!(!dir.join("SingletonCookie").exists());

        // This test process is alive and is not a Chrome on that profile.
        std::os::unix::fs::symlink(format!("host-{}", std::process::id()), dir.join("SingletonLock")).unwrap();
        clear_stale_locks(&dir);
        assert!(std::fs::symlink_metadata(dir.join("SingletonLock")).is_ok(), "a live process that isn't ours is not touched");
        let _ = std::fs::remove_dir_all(&dir);
    }
