use super::*;

    #[test]
    pub(super) fn a_skill_is_explained_only_by_patterns_that_exist() {
        // `<because>` is model-written, so it will sometimes name a page that was
        // never created. Recording it would leave PURPOSE.md pointing at a missing
        // file, which reads as a lost pattern rather than an invented citation.
        let v = scratch();
        let wiki = Wiki::new(&v);
        wiki.upsert_pattern("real-one", "A real pattern", wiki::Kind::Failure, "evidence").unwrap();
        assert!(wiki.read_pattern("real-one").is_some());
        assert!(wiki.read_pattern("never-written").is_none());

        let dir = v.join("skills/release");
        let motives: Vec<String> = ["real-one", "never-written"]
            .iter()
            .filter(|b| wiki.read_pattern(b).is_some())
            .map(|b| b.to_string())
            .collect();
        wiki::write_purpose(&dir, "release", &motives, "Fixed it.").unwrap();

        let text = std::fs::read_to_string(dir.join("PURPOSE.md")).unwrap();
        assert!(text.contains("real-one"));
        assert!(!text.contains("never-written"), "an invented citation was written down:\n{text}");
    }

    #[test]
    pub(super) fn the_wiki_is_outside_what_an_undo_puts_back() {
        // The structural invariant the whole layer rests on. If `Scope` ever grows to
        // cover the wiki, an undo starts erasing the lessons too and the loop goes
        // back to being amnesiac — so this is asserted rather than assumed.
        let (d, v) = (scratch(), scratch());
        let wiki = Wiki::new(&v);
        wiki.upsert_pattern("changelog-step", "Release skips the changelog", wiki::Kind::Failure, "evidence").unwrap();

        let scope = Scope::for_review(&d, &v);
        let snap = snapshot(&scope);
        assert!(
            !snap.files.keys().any(|k| k.contains("/wiki/")),
            "the wiki was snapshotted, so an undo would erase it: {:?}",
            snap.files.keys().collect::<Vec<_>>()
        );

        // A skill changes and the wiki grows; the undo must separate them.
        let skill = d.join("skills/release/SKILL.md");
        std::fs::create_dir_all(skill.parent().unwrap()).unwrap();
        std::fs::write(&skill, "---\nname: release\n---\nnew steps").unwrap();
        wiki.upsert_pattern("changelog-step", "Release skips the changelog", wiki::Kind::Failure, "more evidence").unwrap();

        restore(&snap, &scope);
        assert!(!skill.exists(), "the skill went back");
        let after = wiki.patterns();
        assert_eq!(after.len(), 1, "the pattern survived the undo");
        assert_eq!(after[0].seen, 2, "including the reinforcement written during the pass");
    }

    #[test]
    pub(super) fn an_undone_change_is_shown_to_the_next_review_so_it_is_not_written_again() {
        // The bug this closes: `restore` reverts the skill and `forget_snapshot` drops
        // the record, leaving the next pass with exactly the inputs that produced the
        // edit the user just rejected. It would write it again, forever.
        let (d, v) = (scratch(), scratch());
        let wiki = Wiki::new(&v);
        wiki.upsert_pattern("changelog-step", "Release skips the changelog", wiki::Kind::Failure, "evidence").unwrap();
        wiki.record_impact(&wiki::Impact {
            at: "2026-08-31 10:00".into(),
            target: "release".into(),
            patterns: vec!["changelog-step".into()],
            verdict: wiki::Verdict::Accepted,
            snapshot: "evo-01".into(),
            note: "added a changelog step".into(),
        })
        .unwrap();

        assert!(!build_prompt("t", &d, &v).contains("already rejected"), "nothing rejected yet");

        // What `POST /v1/journal/undo` does.
        assert_eq!(wiki.mark_undone("evo-01"), 1);

        let prompt = build_prompt("t", &d, &v);
        assert!(prompt.contains("already rejected"), "the next review is warned:\n{prompt}");
        assert!(prompt.contains("added a changelog step"), "and told what not to redo:\n{prompt}");
        assert!(prompt.contains("changelog-step"), "while the pattern itself still stands");
    }

    #[test]
    pub(super) fn the_review_is_shown_what_earlier_passes_learned() {
        let (d, v) = (scratch(), scratch());
        assert!(!build_prompt("t", &d, &v).contains("earlier reviews"), "a first run has no history to show");

        let wiki = Wiki::new(&v);
        for _ in 0..3 {
            wiki.upsert_pattern("changelog-step", "Release skips the changelog", wiki::Kind::Failure, "evidence").unwrap();
        }
        let prompt = build_prompt("the transcript", &d, &v);
        assert!(prompt.contains("changelog-step"));
        assert!(prompt.contains("seen 3\u{d7}"), "recurrence is what earns a skill change:\n{prompt}");
        assert!(prompt.contains("the transcript"), "and the conversation is still there");
    }

    #[test]
    pub(super) fn a_pass_that_only_learned_something_changes_no_file_the_user_owns() {
        // `Noted` has to stay invisible to the user: a pattern is what the next review
        // inherits, not work anyone asked for or has to check.
        let (d, v) = (scratch(), scratch());
        let scope = Scope::for_review(&d, &v);
        let snap = snapshot(&scope);
        wiki::commit(
            &v,
            &[("changelog-step".into(), "Release skips the changelog".into(), wiki::Kind::Failure, "evidence".into())],
            &[],
            "noted changelog-step",
        );
        assert!(changed_since(&snap, &scope).is_empty(), "nothing in the user's skills or memory moved");
        assert_eq!(Wiki::new(&v).patterns().len(), 1, "but the wiki grew");
    }

    #[test]
    pub(super) fn a_truncated_or_empty_edit_block_is_dropped_not_written() {
        // An answer cut off mid-file would otherwise overwrite a skill with a fragment.
        assert!(parse_review("<edit path=\"skills/a/SKILL.md\">half a fi").is_none());
        assert!(parse_review("<edit path=\"skills/a/SKILL.md\">   </edit>").is_none());
        assert!(parse_review("<edit path=\"\">content</edit>").is_none());
    }

    #[test]
    pub(super) fn only_skills_and_core_memory_are_writable() {
        let d = scratch();
        let v = d.join("vault");
        assert!(resolve_path(&d, &v, "skills/release/SKILL.md").is_ok());
        assert!(resolve_path(&d, &v, "journal/MEMORY.md").is_ok());

        // Everything a wandering review might reach for.
        for bad in [
            "../../../etc/passwd",
            "/etc/passwd",
            "skills/../../secrets.json",
            "skills/a/b/SKILL.md",
            "skills/SKILL.md",
            "journal/daily/2026-08-12.md",
            "settings.json",
            "",
            "   ",
        ] {
            assert!(resolve_path(&d, &v, bad).is_err(), "must refuse {bad:?}");
        }
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    pub(super) fn a_review_may_append_to_core_memory_but_never_rewrite_it() {
        let before = "- Prefers concise answers.\n- Ships on Fridays.";

        // One new bullet, everything kept: allowed.
        assert!(guard_core_edit(before, "- Prefers concise answers.\n- Ships on Fridays.\n- Uses Postgres.").is_ok());
        // No change at all is fine too.
        assert!(guard_core_edit(before, before).is_ok());

        // Dropping a fact is Deep Dream's job, not a review's — this is how long-term
        // memory would quietly disappear.
        let err = guard_core_edit(before, "- Prefers concise answers.").unwrap_err();
        assert!(err.contains("may not remove"), "got: {err}");
        // A wholesale rewrite loses the originals and is refused for the same reason.
        assert!(guard_core_edit(before, "- Something else entirely.").is_err());
        // Even keeping everything, a flood of new entries is refused.
        assert!(guard_core_edit(before, &format!("{before}\n- a\n- b\n- c")).is_err());
    }

    #[test]
    pub(super) fn a_written_skill_must_actually_be_a_skill() {
        let good = "---\nname: release\ndescription: Cut a release\n---\n\nSteps here.";
        assert!(guard_skill_edit("release", good).is_ok());
        // Prose with no frontmatter would be silently skipped at load time — worse
        // than never writing it, because the user would think a skill exists.
        assert!(guard_skill_edit("release", "Just some notes about releasing.").is_err());
        assert!(guard_skill_edit("release", "---\nname: release\n---\n").is_err());
    }
