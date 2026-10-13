use super::*;

    #[test]
    pub(super) fn a_pass_is_fully_reversible_including_files_it_created() {
        let d = scratch();
        let v = d.join("vault");
        let journal = Journal::new(&v);
        journal.write_core("- Original fact.").unwrap();
        let skill = d.join("skills/release/SKILL.md");
        std::fs::create_dir_all(skill.parent().unwrap()).unwrap();
        std::fs::write(&skill, "original skill").unwrap();

        let scope = Scope::for_review(&d, &v);
        let snap = snapshot(&scope);

        // The pass edits one file and creates another.
        journal.write_core("- Rewritten fact.").unwrap();
        let created = d.join("skills/brand-new/SKILL.md");
        std::fs::create_dir_all(created.parent().unwrap()).unwrap();
        std::fs::write(&created, "invented skill").unwrap();

        let changed = changed_since(&snap, &scope);
        assert_eq!(changed.len(), 2, "both the edit and the creation are seen");

        assert_eq!(restore(&snap, &scope), 2);
        assert_eq!(journal.read_core().trim(), "- Original fact.");
        assert!(!created.exists(), "a file the pass created is removed, not left behind");
        assert_eq!(std::fs::read_to_string(&skill).unwrap(), "original skill");
        assert!(changed_since(&snap, &scope).is_empty(), "back to the snapshot exactly");
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    pub(super) fn undo_leaves_the_rest_of_the_journal_alone() {
        let d = scratch();
        let v = d.join("vault");
        let journal = Journal::new(&v);
        journal.write_core("- Original fact.").unwrap();
        let scope = Scope::for_review(&d, &v);
        let snap = snapshot(&scope);

        // The pass edits core memory…
        journal.write_core("- Original fact.\n- A new fact.").unwrap();
        // …and then, after it, unrelated things land in the journal: a daily entry
        // from another conversation, the nightly dream, and the pass's own record.
        journal.append_daily("2026-08-13", "a conversation elsewhere ended", "").unwrap();
        journal.write_dream("2026-08-13", "nightly consolidation ran").unwrap();
        record(&v, "I learned something", &["MEMORY.md".into()]).unwrap();

        assert_eq!(restore(&snap, &scope), 1, "only core memory is put back");
        assert_eq!(journal.read_core().trim(), "- Original fact.");
        // None of these are the review's to revert — reverting them would lose real
        // memory and erase the evidence that the pass ever happened.
        assert!(journal.read_daily("2026-08-13").is_some(), "daily memory survives an undo");
        assert!(!journal.dreams().is_empty(), "the dream diary survives an undo");
        assert!(!journal.evolutions().is_empty(), "the record of the pass survives its own undo");
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    pub(super) fn an_unchanged_tree_reports_no_changes() {
        let d = scratch();
        let v = d.join("vault");
        Journal::new(&v).write_core("- A fact.").unwrap();
        let scope = Scope::for_review(&d, &v);
        let snap = snapshot(&scope);
        assert!(changed_since(&snap, &scope).is_empty());
        // Rewriting identical content is not a change — this is what stops a
        // no-op pass from notifying the user.
        Journal::new(&v).write_core("- A fact.").unwrap();
        assert!(changed_since(&snap, &scope).is_empty());
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    pub(super) fn snapshots_are_persisted_and_the_latest_wins() {
        let d = scratch();
        let v = d.join("vault");
        Journal::new(&v).write_core("- one").unwrap();
        let scope = Scope::for_review(&d, &v);
        let mut a = snapshot(&scope);
        a.taken_at = 100.0;
        save_snapshot(&d, &a);
        let mut b = snapshot(&scope);
        b.taken_at = 200.0;
        save_snapshot(&d, &b);
        assert_eq!(latest_snapshot(&d).unwrap().id, b.id, "undo targets the most recent pass");
        std::fs::remove_dir_all(&d).ok();
    }

    /// The whole loop against a real local model: a transcript where a skill
    /// demonstrably misfired should produce an edit to that skill, a record, and a
    /// snapshot that puts everything back.
    #[tokio::test]
    #[ignore = "requires Ollama with a chat model"]
    pub(super) async fn e2e_a_skill_that_misfired_gets_fixed_and_can_be_undone() {
        let d = scratch();
        let v = d.join("vault");
        let skill = d.join("skills/release/SKILL.md");
        std::fs::create_dir_all(skill.parent().unwrap()).unwrap();
        let original = "---\nname: release\ndescription: Cut a release\n---\n\n# Release\n\n1. Run the tests.\n2. Publish the package.\n";
        std::fs::write(&skill, original).unwrap();

        let transcript = "user: cut a release for me\n\n\
            assistant: Done — ran the tests and published the package.\n\n\
            user: you forgot to tag the commit again. every single release you skip the git tag. \
            the release skill needs a tag step between running tests and publishing, otherwise the \
            published version can't be traced back to a commit.\n\n\
            assistant: You're right, I'll tag it now.";

        // Deliberately not the default 3B: a review rewrites files that shape every
        // future conversation, and a model that small echoes its prompt instead of
        // emitting the edit format. See `MIN_CAPABLE` below.
        let s = Settings { ollama_model: "qwen2.5:7b-instruct".into(), ..Settings::default() };
        let out = review(&s, &d, &v, transcript).await;

        let scope = Scope::for_review(&d, &v);
        match out {
            Outcome::Changed { snapshot, files, .. } => {
                assert!(!files.is_empty());
                let after = std::fs::read_to_string(&skill).unwrap();
                assert_ne!(after, original, "the skill should have been rewritten");
                assert!(after.to_lowercase().contains("tag"), "the fix should mention tagging:\n{after}");

                // Recorded next to the dream diaries.
                assert!(!Journal::new(&v).evolutions().is_empty(), "the pass should leave a record");

                // And fully reversible.
                let snap = latest_snapshot(&d).expect("a snapshot was saved");
                assert_eq!(snap.id, snapshot);
                restore(&snap, &scope);
                assert_eq!(std::fs::read_to_string(&skill).unwrap(), original, "undo restores the original exactly");
            }
            other => panic!("expected a change, got {other:?}"),
        }
        std::fs::remove_dir_all(&d).ok();
    }

    /// The property that matters more than any single fix: small talk changes nothing.
    #[tokio::test]
    #[ignore = "requires Ollama with a chat model"]
    pub(super) async fn e2e_ordinary_chat_changes_nothing() {
        let d = scratch();
        let v = d.join("vault");
        let s = Settings::default();
        let out = review(
            &s, &d, &v,
            "user: morning\n\nassistant: Morning! What are you working on?\n\nuser: nothing much, just catching up on email\n\nassistant: Sounds good — shout if you want a hand.",
        ).await;
        assert_eq!(out, Outcome::Silent, "small talk must not change anything");
        std::fs::remove_dir_all(&d).ok();
    }

    #[tokio::test]
    pub(super) async fn an_empty_transcript_never_calls_the_model() {
        let d = scratch();
        let v = d.join("vault");
        // Unreachable Ollama: a Failed result would prove it tried to call out.
        let s = Settings { ollama_url: "http://127.0.0.1:1".into(), ..Settings::default() };
        assert_eq!(review(&s, &d, &v, "   \n ").await, Outcome::Silent);
        std::fs::remove_dir_all(&d).ok();
    }
