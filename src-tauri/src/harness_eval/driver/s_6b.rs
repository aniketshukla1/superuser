//! §6b: three statistics, not one

use super::*;

    pub(super) fn oc(task: &'static str, run: usize, v: Verdict) -> Outcome {
        Outcome {
            task, group: "read", model: "m".into(), run, verdict: v,
            why: if v == Verdict::Pass { None } else { Some("x".into()) },
            cause: if v == Verdict::Pass { None } else { Some(Cause::Model) },
            steps: 1, seconds: 1.0, input_tokens: 0, output_tokens: 0, cost_usd: None, not_comparable: None,
        }
    }

    #[test]
    pub(super) fn pass_at_one_and_pass_k_are_different_numbers_and_both_are_shown() {
        // Two tasks, three runs. `steady` passes all three; `flaky` passes one.
        // pass@1 = 4/6 = 67%. pass^k = 1/2 = 50%. Reporting only the total (4/6)
        // is the thing §6b calls dishonest: it reads as pass@1 and hides that half
        // the suite is unreliable rather than merely imperfect.
        let mut o = Vec::new();
        for r in 0..3 { o.push(oc("steady", r, Verdict::Pass)); }
        o.push(oc("flaky", 0, Verdict::Pass));
        o.push(oc("flaky", 1, Verdict::Fail));
        o.push(oc("flaky", 2, Verdict::Fail));
        let t = report(&o);
        assert!(t.contains("67%"), "pass@1 missing: {t}");
        assert!(t.contains("1/2 (50%)"), "pass^k missing: {t}");
    }

    #[test]
    pub(super) fn a_blank_run_does_not_cost_a_task_its_pass_k() {
        // "A blank is not a zero" — the rule this file applies to cost and to the
        // denominator applies here too, or a driver timeout would look like
        // unreliability in the model.
        let mut o = vec![oc("t", 0, Verdict::Pass), oc("t", 1, Verdict::Pass)];
        let mut blank = oc("t", 2, Verdict::Fail);
        blank.verdict = Verdict::NoResult;
        o.push(blank);
        let t = report(&o);
        assert!(t.contains("1/1 (100%)"), "a blank should not break an all-pass task: {t}");
    }
