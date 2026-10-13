use super::*;

    pub(super) fn rec(answer: &str) -> TurnRecord {
        TurnRecord { answer: answer.into(), steps: 1, ..Default::default() }
    }

    pub(super) fn task(id: &str) -> &'static Task {
        TASKS.iter().find(|t| t.id == id).expect("task exists")
    }

    /// A run is marked only when it had a browser the board's rows didn't:
    /// the app-wide switch on and the assistant holding its own — or the switch on and the
    /// assistant unreadable, which is reported rather than assumed away.
    #[test]
    pub(super) fn a_run_is_marked_only_when_it_had_a_browser_the_board_did_not() {
        use serde_json::json;
        let (on, off) = (json!({"browser_enabled": true}), json!({"browser_enabled": false}));
        let (with, without) = (json!({"tools": ["read_file", "browser"]}), json!({"tools": ["read_file"]}));
        assert_eq!(browser_note(&off, Some(&with)), None);
        assert_eq!(browser_note(&json!({}), Some(&with)), None, "an app from before the browser offers none");
        assert_eq!(browser_note(&on, Some(&without)), None, "switched on, and this assistant has none of its own");
        assert!(browser_note(&on, Some(&with)).unwrap().contains("had its own"));
        assert!(browser_note(&on, None).unwrap().contains("couldn't be read"));
    }

    #[test]
    pub(super) fn scoring_carries_why_a_run_is_not_comparable() {
        let mut r = rec("yes");
        r.not_comparable = Some("ran with the browser on".into());
        assert_eq!(score(task("one_word"), "m", &r, 24).not_comparable.as_deref(), Some("ran with the browser on"));
        r.unreachable = Some("timed out".into());
        assert_eq!(score(task("one_word"), "m", &r, 24).not_comparable.as_deref(), Some("ran with the browser on"), "a blank keeps it too");
        assert_eq!(score(task("one_word"), "m", &rec("yes"), 24).not_comparable, None);
    }

    /// Marked where it is read: the cell, the model's summary line, and a note beneath saying
    /// how many of that model's runs and why. Counted, not dropped. Nothing is marked when
    /// nothing is flagged.
    #[test]
    pub(super) fn the_report_marks_runs_that_are_not_comparable_and_says_why() {
        let mk = |task: &'static str, model: &str, note: Option<&str>| Outcome {
            task, group: "instruction", model: model.into(), run: 0, verdict: Verdict::Pass, why: None, cause: None,
            steps: 1, seconds: 1.0, input_tokens: 1, output_tokens: 1, cost_usd: None, not_comparable: note.map(str::to_string),
        };
        let out = report(&[
            mk("one_word", "a", Some("ran with the browser on: x")),
            mk("json_only", "a", None),
            mk("one_word", "b", None),
            mk("json_only", "b", None),
        ]);
        assert!(out.contains("| `one_word` | instruction | ✅† | ✅ |"), "{out}");
        assert!(out.contains("| `json_only` | instruction | ✅ | ✅ |"), "{out}");
        assert!(out.contains("| a† | **2/2**"), "counted, not dropped:\n{out}");
        assert!(out.contains("| b | **2/2**"), "{out}");
        assert!(out.contains("- `a`: 1 of 2 runs ran with the browser on: x."), "{out}");
        assert!(!out.contains("- `b`:"), "{out}");
        assert!(out.contains("scratch instance (`SUPERUSER_DATA_DIR`)"), "{out}");

        let clean = report(&[mk("one_word", "a", None), mk("json_only", "b", None)]);
        assert!(!clean.contains('†'), "{clean}");
    }

    #[test]
    pub(super) fn a_task_with_a_fixture_tells_the_model_the_fixture_exists() {
        // The bug this pins. `cites_what_it_read` shipped a fixture and a prompt —
        // "summarise the launch date and name the file you got it from" — that never
        // said a file existed. Both models duly answered from somewhere else: one read
        // the date off the system prompt's own clock, one searched its memory, one
        // invented a path under ~/Documents. 0/3 on both, and not one of those runs
        // was about citation. `read_named_file` passes 3/3 on the same fixture, and
        // the only difference is that it says which file.
        //
        // So: if a task plants files, the prompt must either name one or locate the
        // model in the folder. Otherwise the task measures whatever the model guesses.
        for t in TASKS.iter() {
            if t.fixture.is_empty() {
                continue;
            }
            // The one deliberate exception: its fixture is a *distractor*, planted so
            // the check can catch a model inventing contents for a file that is not
            // there. Naming it would defeat the point.
            if t.id == "admits_a_missing_file" {
                continue;
            }
            let p = t.prompt.to_lowercase();
            let names_a_fixture = t.fixture.iter().any(|(n, _)| p.contains(&n.to_lowercase()));
            let locates_the_model = p.contains(" here") || p.contains("this folder");
            assert!(
                names_a_fixture || locates_the_model,
                "{}: plants {} file(s) and the prompt mentions none of them — the model \
                 has no way to know they exist: {:?}",
                t.id,
                t.fixture.len(),
                t.prompt
            );
        }
    }

    #[test]
    pub(super) fn no_task_answer_is_findable_in_this_source_file() {
        // The bug this pins: the haystack marker was a literal here, so a model whose
        // shell reached the repository found it in `harness_eval.rs` and answered with
        // that filename instead of the fixture's. Guarding against the model's memory
        // and not against the filesystem is half a guard.
        let me = crate::test_support::module_source("harness_eval");
        // Both splits. The held-out set needs this guard more than the feedback set
        // does, not less: a leaked answer there does not just cost one task, it makes
        // the generalization number itself untrustworthy.
        for t in TASKS.iter().chain(HELD_OUT.iter()) {
            // Vault fixtures are held to the same rule as working-folder ones. They
            // need it more, if anything: a memory's whole job is to be the only place
            // an answer lives, so an answer spelled out here is an answer the model
            // can reach without the brain working at all.
            for (_, body) in t.fixture.iter().chain(t.vault.iter()) {
                assert!(
                    body.contains(MARKER_SLOT) || !body.contains("marker"),
                    "{}: a fixture that names a marker must template it, not spell it",
                    t.id
                );
            }
        }
        // And the template really is a template — the token itself lives nowhere here.
        //
        // Split with `concat!` on purpose, and the first version of this assertion is
        // the reason: written as one literal it put the very string it forbids into
        // the file it scans, and failed against itself. A test that searches its own
        // source cannot spell what it is searching for.
        assert!(me.contains(MARKER_SLOT), "the slot should be present");
        assert!(
            !me.contains(concat!("ZEBRA", "QUARTZ")),
            "the old literal must not come back"
        );
    }

    #[test]
    pub(super) fn a_task_with_a_vault_tells_the_model_to_look_in_it() {
        // The sibling of the fixture rule above, and the same bug waiting to happen.
        // A memory the model is never told to consult is a memory it will answer
        // around — and the task then measures whether the model guesses, which is
        // what `cites_what_it_read` spent three phases doing.
        for t in TASKS.iter().chain(HELD_OUT.iter()) {
            if t.vault.is_empty() {
                continue;
            }
            let p = t.prompt.to_lowercase();
            // The exception is the one task whose subject *is* the standing rule:
            // telling it to check its memory would hand it the answer.
            if t.id == "applies_a_stated_preference" {
                continue;
            }
            assert!(
                p.contains("remember") || p.contains("memory") || p.contains("recall"),
                "{}: seeds {} memories and never tells the model they exist: {:?}",
                t.id,
                t.vault.len(),
                t.prompt
            );
        }
    }

    #[test]
    pub(super) fn a_vault_fixture_is_a_file_the_engine_can_actually_parse() {
        // A fixture with malformed frontmatter does not fail loudly — `parse_file`
        // falls back to defaults, so the memory silently loses its date and its
        // origin. `prefers_the_newer_fact` would then be unfalsifiable: both halves
        // undated, and the task red for a reason that has nothing to do with the
        // harness. Parse them here, where it is cheap to be sure.
        let d = std::env::temp_dir().join(format!("bba-vaultfix-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let eng = crate::core::memory::MemoryEngine::new(d.clone());
        for t in TASKS.iter().chain(HELD_OUT.iter()) {
            for (name, body) in t.vault {
                std::fs::write(d.join(name), body.replace(MARKER_SLOT, "TOKEN")).unwrap();
                let m = eng.parse_file(&d.join(name)).unwrap_or_else(|| panic!("{}: {name} did not parse", t.id));
                assert!(!m.title.is_empty(), "{}: {name} has no title", t.id);
                assert!(m.date.starts_with("20"), "{}: {name} has no usable date: {:?}", t.id, m.date);
                std::fs::remove_file(d.join(name)).unwrap();
            }
        }
        std::fs::remove_dir_all(&d).ok();
    }
