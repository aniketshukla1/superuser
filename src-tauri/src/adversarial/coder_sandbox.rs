use crate::core::coder;

    #[test]
    fn create_cannot_write_through_a_symlinked_directory() {
        // `create` rejects ".." and absolute paths, then joins onto the root and
        // writes — it never canonicalises the result. A symlink inside the
        // workspace is a legal relative path that lands outside it.
        let root = super::scratch("coder-root");
        let outside = super::scratch("coder-outside");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&outside, root.join("link")).unwrap();

        let r = coder::create(root.to_str().unwrap(), "link/pwned.txt", "escaped");
        let escaped = outside.join("pwned.txt").exists();
        std::fs::remove_dir_all(&root).ok();
        std::fs::remove_dir_all(&outside).ok();
        assert!(!escaped, "wrote outside the workspace through a symlink (result={r:?})");
    }

    #[test]
    fn create_rejects_dotdot_in_every_spelling() {
        let root = super::scratch("coder-dd");
        for rel in ["..%2Fescape.txt", "a/../../escape.txt", "./../escape.txt", "a/..\u{0000}/escape.txt"] {
            let _ = coder::create(root.to_str().unwrap(), rel, "x");
        }
        let parent = root.parent().unwrap().to_path_buf();
        let leaked = parent.join("escape.txt").exists();
        std::fs::remove_dir_all(&root).ok();
        std::fs::remove_file(parent.join("escape.txt")).ok();
        assert!(!leaked, "a '..' spelling escaped the workspace");
    }

    #[test]
    fn a_nul_byte_in_a_path_is_rejected_not_panicked_on() {
        let root = super::scratch("coder-nul");
        let r = std::panic::catch_unwind(|| coder::create(root.to_str().unwrap(), "a\u{0000}b.txt", "x"));
        std::fs::remove_dir_all(&root).ok();
        assert!(r.is_ok(), "a NUL in a path panicked instead of erroring");
    }
