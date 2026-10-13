use super::*;

#[test]
    pub(super) fn an_empty_old_str_appends() {
        // append_keeps_the_rest failed 3/3 on exactly this call, refused as
        // "old_str must not be empty", and the model stopped every time.
        let d = tmp();
        std::fs::write(d.join("list.md"), "one\ntwo\nthree\n").unwrap();
        // `str_replace` returns the new content and the *caller* writes it. The first
        // draft of this test read the file back, saw it unchanged, and reported the
        // append broken — when this function has never written anything.
        let (_, after) = str_replace(d.to_str().unwrap(), "list.md", "", "four").unwrap();
        assert_eq!(after, "one\ntwo\nthree\nfour\n");
        // A file without a trailing newline gets one before the new line, not a join.
        std::fs::write(d.join("b.md"), "one\ntwo").unwrap();
        let (_, after) = str_replace(d.to_str().unwrap(), "b.md", "", "three").unwrap();
        assert_eq!(after, "one\ntwo\nthree\n");
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    pub(super) fn appending_to_an_empty_file_points_at_write_file() {
        let d = tmp();
        std::fs::write(d.join("e.md"), "").unwrap();
        let err = str_replace(d.to_str().unwrap(), "e.md", "", "x").unwrap_err();
        assert!(err.contains("write_file"), "{err}");
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    pub(super) fn a_spacing_mismatch_still_lands_when_it_is_unambiguous() {
        // edit_without_clobbering: old_str "timeout=30", file "timeout = 30".
        let d = tmp();
        std::fs::write(d.join("config.txt"), "retries = 5\ntimeout = 30\nverbose = true\n").unwrap();
        let (_, after) = str_replace(d.to_str().unwrap(), "config.txt", "timeout=30", "timeout = 60").unwrap();
        assert_eq!(after, "retries = 5\ntimeout = 60\nverbose = true\n", "the rest must be untouched");
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    pub(super) fn a_loose_edit_keeps_the_files_own_spacing() {
        // The regression the verification sweep found: the model asked for
        // `timeout=30` -> `timeout=60`, the loose match found `timeout = 30`, and the
        // first version wrote `timeout=60` over it — the edit landed, the spacing
        // changed, and the task ("leave everything else exactly as it is") failed.
        let d = tmp();
        std::fs::write(d.join("config.txt"), "retries = 5\ntimeout = 30\nverbose = true\n").unwrap();
        let (_, after) = str_replace(d.to_str().unwrap(), "config.txt", "timeout=30", "timeout=60").unwrap();
        assert_eq!(after, "retries = 5\ntimeout = 60\nverbose = true\n");
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    pub(super) fn respacing_maps_the_changed_middle_and_nothing_else() {
        assert_eq!(respace("port=8080", "port = 8080", "port=9000"), "port = 9000");
        // Different indentation in the file than in the model's copy is kept.
        assert_eq!(respace("  foo()\n  bar()", "    foo()\n    bar()", "  foo()\n  baz()"), "    foo()\n    baz()");
        // A model that deliberately re-spaced is taken at its word.
        assert_eq!(respace("timeout=30", "timeout = 30", "timeout = 60"), "timeout = 60");
    }

    #[test]
    pub(super) fn a_loose_match_in_two_places_is_refused_not_guessed() {
        let d = tmp();
        std::fs::write(d.join("dup.conf"), "port = 1\nport = 1\n").unwrap();
        let err = str_replace(d.to_str().unwrap(), "dup.conf", "port=1", "port = 2").unwrap_err();
        assert!(err.contains("2 places"), "ambiguity must be reported, never resolved by guessing: {err}");
        assert_eq!(std::fs::read_to_string(d.join("dup.conf")).unwrap(), "port = 1\nport = 1\n");
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    pub(super) fn a_multi_match_edit_says_where_the_matches_are() {
        // "appears 3 times" leaves the model searching for the one it meant. The
        // line numbers turn the retry into a one-step fix.
        let d = tmp();
        std::fs::write(d.join("app.conf"), "port = 1\nname = x\nport = 2\nother\nport = 3\n").unwrap();
        let err = str_replace(d.to_str().unwrap(), "app.conf", "port =", "PORT =").unwrap_err();
        assert!(err.contains("appears 3 times"), "{err}");
        assert!(err.contains("lines 1, 3, 5"), "it must name where: {err}");
        assert!(err.contains("surrounding context"), "and the move: {err}");
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    pub(super) fn a_great_many_matches_are_counted_rather_than_all_listed() {
        let d = tmp();
        std::fs::write(d.join("many.txt"), "x\n".repeat(40)).unwrap();
        let err = str_replace(d.to_str().unwrap(), "many.txt", "x", "y").unwrap_err();
        assert!(err.contains("and 34 more"), "{err}");
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    pub(super) fn a_missing_file_may_simply_be_missing() {
        // The regression this clause exists for. The first teaching version said
        // "use list_dir or grep — do not guess another path", full stop, and a model
        // asked about a file that does not exist went hunting instead of answering:
        // `admits_a_missing_file` fell 3/3 to 1/3 on the sweep that shipped it.
        let d = tmp();
        std::fs::write(d.join("present.md"), "x").unwrap();
        let err = read(d.to_str().unwrap(), "absent.md").unwrap_err();
        assert!(err.contains("simply not there"), "absence has to be an available answer: {err}");
        assert!(err.contains("list_dir"), "and hunting is still offered: {err}");
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    pub(super) fn a_missing_path_names_what_is_actually_there() {
        // The dead end this replaces: "no such path: No such file or directory
        // (os error 2)" told the model its guess was wrong and nothing about what
        // would have been right. Measured: nine read_file calls, nine invented
        // paths, one list_dir in 59 sessions.
        let d = tmp();
        std::fs::create_dir_all(d.join("reports")).unwrap();
        std::fs::write(d.join("reports/q3.csv"), "a,b\n").unwrap();
        std::fs::write(d.join("README.md"), "hi").unwrap();

        let err = read(d.to_str().unwrap(), "reports/q4-final.csv").unwrap_err();
        assert!(err.contains("no such path"), "{err}");
        assert!(err.contains("`reports`"), "it must name the directory that does exist: {err}");
        assert!(err.contains("q3.csv"), "and show what is in it: {err}");
        assert!(err.contains("list_dir") && err.contains("grep"), "and name the move: {err}");
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    pub(super) fn a_guess_with_no_real_ancestor_falls_back_to_the_workspace_root() {
        let d = tmp();
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("only.txt"), "x").unwrap();
        let err = read(d.to_str().unwrap(), "made/up/deep/thing.csv").unwrap_err();
        assert!(err.contains("only.txt"), "should fall back to listing the root: {err}");
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    pub(super) fn the_listing_is_capped_so_a_wrong_guess_is_not_a_directory_dump() {
        let d = tmp();
        std::fs::create_dir_all(&d).unwrap();
        for i in 0..40 {
            std::fs::write(d.join(format!("f{i:02}.txt")), "x").unwrap();
        }
        let err = read(d.to_str().unwrap(), "nope.txt").unwrap_err();
        assert!(err.contains("and 28 more"), "the rest must be counted, not listed: {err}");
        assert!(err.len() < 600, "this lands in context on every wrong guess: {} chars", err.len());
        std::fs::remove_dir_all(&d).ok();
    }

    // One directory per call. Keying only on the pid made every test share a
    // directory that each `tmp()` wiped — so parallel tests raced and randomly
    // failed with "bad workspace: No such file or directory".
    pub(super) fn tmp() -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(0);
        let p = std::env::temp_dir().join(format!(
            "bba_coder_test_{}_{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        std::fs::canonicalize(&p).unwrap()
    }

    #[test]
    pub(super) fn str_replace_requires_unique_match() {
        let root = tmp();
        std::fs::write(root.join("a.txt"), "foo\nbar\nfoo\n").unwrap();
        let rs = root.to_string_lossy();
        // 0 matches → error
        assert!(str_replace(&rs, "a.txt", "zzz", "x").is_err());
        // 2 matches → error (not unique)
        assert!(str_replace(&rs, "a.txt", "foo", "x").is_err());
        // 1 match → ok, returns (old, new)
        let (old, new) = str_replace(&rs, "a.txt", "bar", "BAZ").unwrap();
        assert_eq!(old, "foo\nbar\nfoo\n");
        assert_eq!(new, "foo\nBAZ\nfoo\n");
    }
