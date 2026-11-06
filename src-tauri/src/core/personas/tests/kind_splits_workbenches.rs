use super::*;

#[test]
    pub(super) fn kind_splits_workbenches_from_assistants() {
        let r = PersonaRegistry::load(std::path::Path::new("/nonexistent"));
        // The shipped three land on the right side of the line.
        assert_eq!(r.manifest("assistant").unwrap().kind, "assistant");
        assert_eq!(r.manifest("coder").unwrap().kind, "workbench");
        // ops is family: knowledge and still a workbench, because it SAYS so. It is
        // the case that always disproved "shell implies workbench" and was outvoted
        // by its own explicit `kind:` line.
        assert_eq!(r.manifest("ops").unwrap().kind, "workbench");

        // Undeclared manifests are inferred from the FAMILY. A shell no longer
        // implies a workbench: the assistants all have one and none of them is bound
        // to a repo, which is what `kind` actually decides.
        let shelly = parse_manifest(
            "---\nid: x\nname: X\ntools: [files, shell]\n---\nbody",
            Some("x"), false, None,
        ).unwrap();
        assert_eq!(shelly.kind, "assistant", "a shell is a capability, not a place");
        let coder_like = parse_manifest(
            "---\nid: c\nname: C\nfamily: code\ntools: [files]\n---\nbody",
            Some("c"), false, None,
        ).unwrap();
        assert_eq!(coder_like.kind, "workbench", "family: code is what binds it to a place");
        let plain = parse_manifest(
            "---\nid: y\nname: Y\ntools: [web, files]\n---\nbody",
            Some("y"), false, None,
        ).unwrap();
        assert_eq!(plain.kind, "assistant");
    }

    #[test]
    pub(super) fn a_workbench_that_cannot_work_anywhere_is_refused() {
        // `kind` is not a label. The GUI acts on it: picking a workbench flips the
        // sidebar to the project list, where an assistant that has no folder appears
        // nowhere at all — so the chat area switches, the sidebar shows somebody
        // else's repos, and the click reads as ignored. That is the bug this rejects.
        let err = parse_manifest(
            "---\nid: z\nname: Z\nkind: workbench\ntools: [files, web]\n---\nbody",
            Some("z"), false, None,
        )
        .unwrap_err();
        assert!(err.contains("no `shell`"), "got: {err}");

        // Both of the things that legitimately make one are still accepted.
        assert_eq!(
            parse_manifest("---\nid: z\nname: Z\nkind: workbench\ntools: [shell]\n---\nbody", Some("z"), false, None)
                .unwrap()
                .kind,
            "workbench"
        );
        assert_eq!(
            parse_manifest("---\nid: z\nname: Z\nkind: workbench\nfamily: code\ntools: [files]\n---\nbody", Some("z"), false, None)
                .unwrap()
                .kind,
            "workbench"
        );
    }

    #[test]
    pub(super) fn every_shipped_workbench_can_actually_run_something() {
        // The rule, applied to the roster rather than to one parse. Four manifests
        // had drifted the other way — Data Analyst, Finance Analyst, HR Partner and
        // Customer Support all called themselves workbenches with no shell between
        // them, because each was written by copying the frontmatter of a neighbour.
        for p in builtins() {
            if p.kind == "workbench" {
                assert!(
                    p.tools.contains(&"shell".to_string()) || p.family == "code",
                    "'{}' is a workbench that can't run anything and isn't bound to a repo",
                    p.id
                );
            }
            // No converse. It used to assert that anything with a shell must BE a
            // workbench, which was the same conflation the inference made: `kind` is
            // about being bound to a place, and a shell is a capability. Every
            // assistant ships with one now and none of them belongs in the project
            // list.
        }
        // And the four that were wrong are now on the assistant side.
        for id in ["analyst", "finance", "hr", "support"] {
            assert_eq!(builtins().into_iter().find(|p| p.id == id).unwrap().kind, "assistant", "{id}");
        }
    }

    pub(super) fn scratch() -> PathBuf {
        static SCRATCH_N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let seq = SCRATCH_N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let dir = std::env::temp_dir().join(format!("bba-personas-{}-{}-{}", std::process::id(), n, seq));
        let _ = fs::create_dir_all(&dir);
        dir
    }

    #[test]
    pub(super) fn builtins_parse() {
        let b = builtins();
        assert_eq!(b.len(), 20);
        let ops = b.iter().find(|m| m.id == "ops").unwrap();
        assert_eq!(ops.family, "knowledge");
        assert!(ops.tools.contains(&"shell".to_string()));
        assert_eq!(ops.recommends.len(), 3);
        assert_eq!(ops.recommends[0].kind, "connector");
        let coder = b.iter().find(|m| m.id == "coder").unwrap();
        assert_eq!(coder.workspace, "git"); // derived from family: code
    }

    #[test]
    pub(super) fn rejects_unknown_tool() {
        let md = "---\nid: x\ntools: [not_a_tool]\n---\nbody";
        assert!(parse_manifest(md, None, false, None).is_err());
    }

    #[test]
    pub(super) fn rejects_missing_body() {
        assert!(parse_manifest("---\nid: x\n---\n", None, false, None).is_err());
    }

    #[test]
    pub(super) fn derives_id_from_filename() {
        let m = parse_manifest("---\nname: My Bot\n---\nhi", Some("My Persona.md"), false, None).unwrap();
        assert_eq!(m.id, "my-persona-md");
    }

    #[test]
    pub(super) fn every_builtin_persona_names_skills_that_actually_exist() {
        // A persona listing a skill nobody ships gets silence, not an error — the
        // same failure shape as a capability that reaches no persona, and the same
        // reason to catch it here rather than in a session.
        let skills = crate::core::skills::SkillStore::load(std::path::Path::new("/nonexistent"));
        for p in builtins() {
            for want in &p.skills {
                assert!(
                    skills.get(want).is_some(),
                    "persona '{}' lists skill '{want}', which nothing ships",
                    p.id
                );
            }
        }
    }

    #[test]
    pub(super) fn the_assistants_whose_judgement_matters_say_they_need_it() {
        // Replaces `recommended_models`, which named model ids nothing read, nothing
        // declared, and no local-first user could act on. This is a property of the
        // WORK — "a small model judges this badly" — true whatever is installed.
        for id in ["security", "cloud-posture", "dep-audit", "lead", "qa", "analyst"] {
            let p = builtins().into_iter().find(|p| p.id == id).unwrap();
            assert!(p.needs_capable_model, "'{id}' judges things; a small model does that badly");
        }
        // Not simply on for everyone, or it says nothing.
        let generalist = builtins().into_iter().find(|p| p.id == "assistant").unwrap();
        assert!(!generalist.needs_capable_model);
    }

    #[test]
    pub(super) fn every_shipped_assistant_has_a_distinct_job() {
        // A roster whose members describe themselves alike is a picker nobody can
        // choose from — the same reason two skills may not share a description.
        // Checked because writing eleven of these makes the twelfth sound like the
        // eleventh.
        let all = builtins();
        let taglines: std::collections::BTreeSet<String> =
            all.iter().map(|p| p.tagline.to_lowercase()).collect();
        assert_eq!(taglines.len(), all.len(), "two assistants share a tagline");

        for p in &all {
            assert!(!p.tagline.trim().is_empty(), "'{}' has no tagline", p.id);
            assert!(p.description.len() > 40, "'{}' has a description too thin to choose by", p.id);
            assert!(!p.starters.is_empty(), "'{}' offers no opening prompt", p.id);
            assert!(!p.tools.is_empty(), "'{}' can do nothing", p.id);
        }
    }
