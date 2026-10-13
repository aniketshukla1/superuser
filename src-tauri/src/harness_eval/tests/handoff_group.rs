//! the handoff group

use super::*;

    #[test]
    pub(super) fn an_assistant_a_task_installs_is_one_the_app_can_read() {
        // A manifest that doesn't parse loads as nothing, the grant then names an assistant
        // that isn't there, and the task fails for a reason that has nothing to do with the
        // model. Checked here, where it costs nothing.
        for t in TASKS.iter().chain(HELD_OUT.iter()) {
            for (name, text) in t.personas {
                let id = name.trim_end_matches(".md");
                let m = crate::core::personas::parse_manifest(&text.replace(MARKER_SLOT, "TOKEN"), Some(id), false, None)
                    .unwrap_or_else(|e| panic!("{}: {name} doesn't parse: {e}", t.id));
                assert_eq!(m.id, id, "{}: {name} must be the id the grant names", t.id);
            }
            // One of them holds the fact; the others are there to be the wrong door, which
            // is the half of the task that measures judgement rather than plumbing.
            assert!(
                t.personas.is_empty() || t.personas.iter().any(|(_, text)| text.contains(MARKER_SLOT)),
                "{}: installs assistants and none of them knows anything only asking could get",
                t.id
            );
            // A grant with nobody installed to receive it names an assistant this machine
            // may or may not have — which makes the task's result depend on the operator.
            for who in t.asks {
                assert!(
                    t.personas.iter().any(|(n, _)| n.trim_end_matches(".md") == *who),
                    "{}: grants {who}, which it doesn't install",
                    t.id
                );
            }
        }
    }

    #[test]
    pub(super) fn a_handoff_is_graded_on_asking_and_on_what_came_back() {
        let ask = |to: &str| call("ask_coworker", &format!(r#"{{"coworker":"{to}","task":"what is the code"}}"#), "Freeze Desk answered:\n\nMK0001");
        let mut asked = browsed("The Freeze Desk says it's MK0001.", vec![ask("freeze-desk")]);
        assert!(asked_and_used_the_answer(&asked).is_ok());
        // Asked, and then answered something else: the relay is what the task grades.
        asked.answer = "I couldn't find it.".into();
        assert!(asked_and_used_the_answer(&asked).unwrap_err().contains("doesn't carry"));
        // Never asked: the marker cannot have come from anywhere honest.
        let alone = browsed("The code is MK0001.", vec![call("recall_memory", "{}", "nothing")]);
        assert!(asked_and_used_the_answer(&alone).unwrap_err().contains("never asked"));
    }

    #[test]
    pub(super) fn with_two_to_choose_from_it_has_to_reach_the_one_that_knows() {
        let ask = |to: &str, result: &str| call("ask_coworker", &format!(r#"{{"coworker":"{to}","task":"east door code"}}"#), result);
        let right = browsed("It's MK0001, from the Badge Desk.", vec![ask("badge-desk", "Badge Desk answered:\n\nMK0001")]);
        assert!(asked_the_one_that_knows(&right).is_ok());
        // Asked the wrong one and passed its shrug back as the answer.
        let wrong = browsed(
            "The Kitchen Rota doesn't keep door codes.",
            vec![ask("kitchen-rota", "Kitchen Rota answered:\n\nI don't keep those.")],
        );
        assert!(asked_the_one_that_knows(&wrong).unwrap_err().contains("neither of those"));
        // Asked both, in the wrong order, and got there: that is a pass.
        let both = browsed(
            "It's MK0001.",
            vec![ask("kitchen-rota", "Kitchen Rota answered:\n\nnot mine"), ask("badge-desk", "Badge Desk answered:\n\nMK0001")],
        );
        assert!(asked_the_one_that_knows(&both).is_ok());
        assert_eq!(coworkers_asked(&both), vec!["kitchen-rota".to_string(), "badge-desk".to_string()]);
    }

    #[test]
    pub(super) fn work_it_can_do_itself_is_graded_on_doing_it() {
        let t = task("does_not_hand_off_its_own_work");
        assert!((t.check)(&rec("18 + 24 + 31 = 73.")).is_ok());
        let mut handed = rec("The Freeze Desk says 73.");
        handed.calls = vec![call("ask_coworker", r#"{"coworker":"freeze-desk","task":"add 18, 24, 31"}"#, "Freeze Desk answered:\n\n73")];
        assert!((t.check)(&handed).unwrap_err().contains("could do itself"));
        assert!((t.check)(&rec("The total is 72.")).is_err());
    }
