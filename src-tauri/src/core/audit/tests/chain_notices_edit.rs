use super::*;

    pub(super) fn scratch() -> std::path::PathBuf {
        static SCRATCH_N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let seq = SCRATCH_N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let n = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let d = std::env::temp_dir().join(format!("bba-audit-{}-{}-{}", std::process::id(), n, seq));
        let _ = std::fs::create_dir_all(&d);
        d
    }

    #[test]
    pub(super) fn appends_and_reads_newest_first() {
        let dir = scratch();
        assert!(recent(&dir, 10).is_empty()); // no file yet → empty, not an error

        log(&dir, "sync", "github", "issues", "12 new", true);
        log(&dir, "send", "assistant", "slack:#ops", "posted the summary", true);
        log(&dir, "exec", "ops", "shell", "kubectl get pods", false);

        let entries = recent(&dir, 10);
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].kind, "exec"); // newest first
        assert!(!entries[0].ok);
        assert_eq!(entries[2].actor, "github");

        // limit is honored
        assert_eq!(recent(&dir, 2).len(), 2);

        assert!(clear(&dir));
        // The wipe says it happened, and says nothing else.
        let after = recent(&dir, 10);
        assert_eq!(after.len(), 1);
        assert_eq!(after[0].event_name(), "audit.wiped");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    pub(super) fn detail_is_capped_and_survives_bad_lines() {
        let dir = scratch();
        log(&dir, "send", "a", "b", &"x".repeat(5000), true);
        // a corrupt line must not break reading the good ones
        let mut f = OpenOptions::new().create(true).append(true).open(path(&dir)).unwrap();
        writeln!(f, "{{not json").unwrap();
        log(&dir, "sync", "c", "d", "ok", true);

        let entries = recent(&dir, 10);
        assert_eq!(entries.len(), 2);
        assert!(entries.iter().any(|e| e.detail.len() == 500));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    pub(super) fn a_first_version_row_still_reads() {
        let dir = scratch();
        std::fs::write(
            path(&dir),
            "{\"at\":1.0,\"kind\":\"send\",\"actor\":\"s1\",\"target\":\"slack:#ops\",\"detail\":\"hi\",\"ok\":true}\n",
        )
        .unwrap();
        log(&dir, "sync", "github", "issues", "3 new", true);
        let rows = recent(&dir, 10);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[1].event_name(), "send");
        let v = verify(&dir);
        assert!(v.intact, "{v:?}");
        assert_eq!((v.legacy, v.linked), (1, 1));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    pub(super) fn the_chain_notices_an_edit_an_insertion_and_a_removal() {
        let dir = scratch();
        for i in 0..4 {
            log(&dir, "exec", "ops", "shell", &format!("command {i}"), true);
        }
        assert!(verify(&dir).intact);

        let original = std::fs::read_to_string(path(&dir)).unwrap();

        // Edited: one word changed in the middle row.
        std::fs::write(path(&dir), original.replace("command 1", "command X")).unwrap();
        let v = verify(&dir);
        assert!(!v.intact);
        assert!(v.problem.as_deref().unwrap().contains("changed after it was written"), "{v:?}");

        // Removed: a middle row taken out.
        let lines: Vec<&str> = original.lines().collect();
        std::fs::write(path(&dir), format!("{}\n{}\n{}\n", lines[0], lines[2], lines[3])).unwrap();
        let v = verify(&dir);
        assert!(!v.intact);
        assert!(v.problem.as_deref().unwrap().contains("removed or reordered"), "{v:?}");

        // Truncated: the newest row taken off the end.
        std::fs::write(path(&dir), format!("{}\n{}\n{}\n", lines[0], lines[1], lines[2])).unwrap();
        let v = verify(&dir);
        assert!(!v.intact);
        assert!(v.problem.as_deref().unwrap().contains("newest rows are missing"), "{v:?}");

        // Inserted: an unlinked row after linked ones.
        std::fs::write(
            path(&dir),
            format!("{original}{{\"at\":9.0,\"kind\":\"send\",\"actor\":\"x\",\"target\":\"y\",\"detail\":\"z\",\"ok\":true}}\n"),
        )
        .unwrap();
        let v = verify(&dir);
        assert!(!v.intact);
        assert!(v.problem.as_deref().unwrap().contains("carries no link"), "{v:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    pub(super) fn a_new_month_rotates_the_file_and_the_chain_runs_across() {
        let dir = scratch();
        let mut e = Entry::event("action.allowed");
        e.at = 1_754_000_000.0; // 2025-07-31/08-01, whichever side the local zone puts it
        append(&dir, e);
        let mut e = Entry::event("action.allowed");
        e.at = 1_760_000_000.0; // 2025-10
        append(&dir, e);
        assert_eq!(rotated(&dir).len(), 1, "the earlier month moved to its own file");
        assert_eq!(recent(&dir, 10).len(), 2, "and both months still read");
        let v = verify(&dir);
        assert!(v.intact, "{v:?}");
        assert_eq!(v.linked, 2);

        // Retention removes the old month and says so. Its own row is written today,
        // which rotates the October file out in turn — that one is kept.
        assert_eq!(apply_retention(&dir, 30), 1);
        let left: Vec<String> = rotated(&dir).iter().map(|p| p.file_name().unwrap().to_string_lossy().into_owned()).collect();
        assert_eq!(left, vec!["audit-2025-10.jsonl".to_string()], "only the removed month is gone");
        assert!(recent(&dir, 10).iter().any(|r| r.event_name() == "audit.retention"));
        assert!(verify(&dir).intact, "the oldest kept row is the anchor");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    pub(super) fn a_row_from_a_newer_build_with_unknown_fields_still_verifies() {
        let dir = scratch();
        log(&dir, "exec", "ops", "shell", "ls", true);
        // Hand-write a linked row carrying a field this build doesn't know.
        let prev = read_head(&dir).unwrap().hash;
        let unlinked = format!("{{\"at\":2.0,\"kind\":\"x\",\"actor\":\"a\",\"target\":\"\",\"detail\":\"\",\"ok\":true,\"future\":1,\"prev\":\"{prev}\"}}");
        let hash = digest(&unlinked);
        let line = format!("{},\"hash\":\"{hash}\"}}", &unlinked[..unlinked.len() - 1]);
        let mut f = OpenOptions::new().append(true).open(path(&dir)).unwrap();
        writeln!(f, "{line}").unwrap();
        std::fs::write(dir.join(HEAD), serde_json::to_string(&Head { hash, rows: 2, month: month_of(now()) }).unwrap()).unwrap();
        let v = verify(&dir);
        assert!(v.intact, "{v:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    pub(super) fn credentials_never_reach_the_file_but_the_command_stays_readable() {
        let dir = scratch();
        let mut e = Entry::event("action.allowed");
        e.subject = Some(Subject {
            tool: "run_shell".into(),
            command: "curl -H 'Authorization: Bearer sk-ant-api03-abcdefghijklmnopqrstuvwxyz' https://u:hunter22@example.com/x?token=abc123".into(),
            ..Default::default()
        });
        e.detail = "key ghp_abcdefghijklmnopqrstuvwxyz0123 and AKIAABCDEFGHIJKLMNOP".into();
        append(&dir, e);
        let text = std::fs::read_to_string(path(&dir)).unwrap();
        for secret in ["sk-ant-api03", "hunter22", "abc123", "ghp_abcdef", "AKIAABCDEFGHIJKLMNOP"] {
            assert!(!text.contains(secret), "{secret} reached the trail: {text}");
        }
        assert!(text.contains("curl -H"), "the command itself is still there");
        assert!(text.contains("[credential]"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    pub(super) fn redaction_by_key_name_reaches_nested_values() {
        let v = serde_json::json!({"target": "slack:#ops", "text": "the launch code", "meta": {"api_key": "x", "n": 3}});
        let r = redact(&v);
        assert_eq!(r["target"], "slack:#ops");
        assert_eq!(r["text"], "[redacted]");
        assert_eq!(r["meta"]["api_key"], "[redacted]");
        assert_eq!(r["meta"]["n"], 3);
    }
