use super::*;

    #[test]
    pub(super) fn create_and_remove_roundtrip_and_reject_escape() {
        let root = tmp();
        let rs = root.to_string_lossy();
        // create nested file
        create(&rs, "sub/new.txt", "hello").unwrap();
        assert_eq!(std::fs::read_to_string(root.join("sub/new.txt")).unwrap(), "hello");
        // creating again fails
        assert!(create(&rs, "sub/new.txt", "x").is_err());
        // path escape is rejected
        assert!(create(&rs, "../escape.txt", "x").is_err());
        assert!(create(&rs, "/etc/passwd", "x").is_err());
        // remove it
        remove(&rs, "sub/new.txt").unwrap();
        assert!(!root.join("sub/new.txt").exists());
        // removing a missing file errors
        assert!(remove(&rs, "sub/new.txt").is_err());
    }

    #[test]
    pub(super) fn run_capture_returns_output_exit_and_blocks_danger() {
        let root = tmp();
        let rs = root.to_string_lossy();
        let (code, out) = run_capture(&rs, "echo hello", 10, 4000);
        assert_eq!(code, 0);
        assert!(out.contains("hello"));
        // non-zero exit is reported
        let (code, _) = run_capture(&rs, "exit 3", 10, 4000);
        assert_eq!(code, 3);
        // stderr is captured
        let (_, out) = run_capture(&rs, "echo oops 1>&2", 10, 4000);
        assert!(out.contains("oops"));
        // denylist blocks catastrophic commands
        let (code, out) = run_capture(&rs, "sudo rm -rf /", 10, 4000);
        assert_eq!(code, -1);
        assert!(out.contains("blocked"));
    }
