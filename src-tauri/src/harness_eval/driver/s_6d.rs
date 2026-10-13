//! §6d: failure attribution

use super::*;

    pub(super) fn rec(answer: &str) -> TurnRecord {
        TurnRecord { answer: answer.into(), steps: 2, ..Default::default() }
    }

    #[test]
    pub(super) fn a_pass_has_no_cause_to_report() {
        assert_eq!(cause(&rec("fine"), "read", 24, false), None);
    }

    #[test]
    pub(super) fn a_blank_is_not_attributed_to_anyone() {
        // NoResult is excluded from the denominator, so attributing it would put a
        // cause on a run that measured nothing.
        let mut r = rec("");
        r.unreachable = Some("timeout".into());
        assert_eq!(cause(&r, "read", 24, true), None);
    }

    #[test]
    pub(super) fn a_turn_that_errored_is_ours_before_it_is_anything_else() {
        // Ordering matters: this record also has no tool calls, and reporting
        // "no tool" for a turn that never got to call one would be a wrong lead.
        let mut r = rec("");
        r.error = Some("400 does not support thinking".into());
        assert_eq!(cause(&r, "read", 24, true), Some(Cause::Provider));
    }

    #[test]
    pub(super) fn the_step_cliff_is_named_whether_it_is_the_count_or_the_text() {
        let mut over = rec("something");
        over.steps = 25;
        assert_eq!(cause(&over, "read", 24, true), Some(Cause::StepCliff));
        assert_eq!(cause(&rec("Reached the step limit."), "read", 24, true), Some(Cause::StepCliff));
    }

    #[test]
    pub(super) fn a_printed_tool_call_is_a_wire_problem_not_a_wrong_answer() {
        let r = rec(r#"{"name":"read_file","parameters":{"path":"a.md"}}"#);
        assert_eq!(cause(&r, "read", 24, true), Some(Cause::ToolCallAsText));
    }

    #[test]
    pub(super) fn arguments_that_never_parsed_are_recognised_by_the_empty_object() {
        // `read_arguments` hands an unparseable call on as `{}` — see core::argrepair.
        // This is the column that would show whether e693e74 stayed fixed.
        let mut r = rec("I could not do it");
        r.calls = vec![Call { name: "read_file".into(), args: "{}".into(), result: String::new() }];
        assert_eq!(cause(&r, "read", 24, true), Some(Cause::ArgumentHandling));
    }

    #[test]
    pub(super) fn a_call_rejected_for_its_arguments_counts_too() {
        let mut r = rec("no");
        r.calls = vec![Call {
            name: "edit_file".into(),
            args: r#"{"path":"a"}"#.into(),
            result: "error: edit_file is missing a required argument: old_str".into(),
        }];
        assert_eq!(cause(&r, "read", 24, true), Some(Cause::ArgumentHandling));
    }

    #[test]
    pub(super) fn no_tool_on_a_task_that_needed_one_is_selection() {
        let r = rec("The answer is probably 42.");
        assert_eq!(cause(&r, "read", 24, true), Some(Cause::NoToolCalled));
    }

    #[test]
    pub(super) fn an_instruction_task_is_allowed_to_answer_without_tools() {
        // `arithmetic_no_tools` failing is the model being wrong, not the harness
        // withholding anything. Calling that "no tool" would file a model failure
        // under our own defects and inflate the half of the split that matters.
        let r = rec("Seventeen.");
        assert_eq!(cause(&r, "instruction", 24, true), Some(Cause::Model));
    }

    #[test]
    pub(super) fn a_wrong_answer_after_real_work_is_the_model() {
        let mut r = rec("The launch date is 4 October.");
        r.calls = vec![Call {
            name: "read_file".into(),
            args: r#"{"path":"alpha.md"}"#.into(),
            result: "# Alpha\n\nThe Perseus launch date is 3 October 2027.\n".into(),
        }];
        assert_eq!(cause(&r, "read", 24, true), Some(Cause::Model));
    }

    #[test]
    pub(super) fn only_the_model_bucket_is_not_ours() {
        for c in [Cause::Provider, Cause::StepCliff, Cause::ToolCallAsText,
                  Cause::ArgumentHandling, Cause::NoToolCalled, Cause::EmptyAnswer] {
            assert!(c.ours(), "{c:?} should be actionable here");
        }
        assert!(!Cause::Model.ours());
    }

    #[test]
    pub(super) fn the_report_says_which_side_the_failures_came_from() {
        let mk = |task: &'static str, c: Cause| Outcome {
            task, group: "read", model: "m".into(), run: 0,
            verdict: Verdict::Fail, why: Some("x".into()), cause: Some(c),
            steps: 2, seconds: 1.0, input_tokens: 0, output_tokens: 0, cost_usd: None, not_comparable: None,
        };
        let out = report(&[mk("a", Cause::ArgumentHandling), mk("b", Cause::Model), mk("c", Cause::Model)]);
        assert!(out.contains("1 on our side, 2 model-side"), "{out}");
        assert!(out.contains("2 × model"), "{out}");
        assert!(out.contains("[arguments]"), "the failure list names the side too: {out}");
    }
