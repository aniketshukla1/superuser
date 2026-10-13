use crate::harness_eval::*;

pub(super) const RECALL: &[Task] = &[
    // ---- Recall ------------------------------------------------------------
    Task {
        id: "answers_from_a_truncated_trace",
        group: "recall",
        prompt: "Read trace.txt, then read approver.md. Then tell me two things: the RELEASE-TAG \
                 recorded in trace.txt, and who signed off. Give both in your answer.",
        fixture: RECALL_TRACE,
        vault: &[],
        boundaries: None,
        standing: "",
        site: &[],
        personas: &[],
        scratch_only: false,
        asks: &[],
        check: |r| {
            if !r.said("Devika") {
                return Err(format!("did not report the approver: {:?}", r.answer.trim()));
            }
            r.said_marker()
                .then_some(())
                .ok_or(format!("did not report the RELEASE-TAG: {:?}", r.answer.trim()))
        },
    },
    // ---- Brain ---------------------------------------------------------------
    Task {
        id: "recalls_a_stored_code",
        group: "brain",
        prompt: "I need to let a visitor in. What is the temporary door code? Check what you remember.",
        fixture: &[],
        vault: &[("notion-77.md", MEM_BADGE)],
        boundaries: None,
        standing: "",
        site: &[],
        personas: &[],
        scratch_only: false,
        asks: &[],
        check: |r| {
            r.said_marker()
                .then_some(())
                .ok_or(format!("did not recall the door code: {:?}", r.answer.trim()))
        },
    },
    Task {
        id: "prefers_the_current_vendor",
        group: "brain",
        prompt: "Who supplies our storage tier at the moment? Check what you remember.",
        fixture: &[],
        vault: &[("linear-14.md", MEM_VENDOR_OLD), ("linear-15.md", MEM_VENDOR_NEW)],
        boundaries: None,
        standing: "",
        site: &[],
        personas: &[],
        scratch_only: false,
        asks: &[],
        check: |r| {
            if !r.said_marker() {
                return Err(format!("did not name the current vendor: {:?}", r.answer.trim()));
            }
            if r.said("Northwind") {
                return Err(format!("also offered the superseded vendor: {:?}", r.answer.trim()));
            }
            Ok(())
        },
    },
    // ---- Instruction following -------------------------------------------------
    Task {
        id: "leap_year_no_tools",
        group: "instruction",
        prompt: "How many days are in a leap year? Reply with only the number.",
        fixture: &[],
        vault: &[],
        boundaries: None,
        standing: "",
        site: &[],
        personas: &[],
        scratch_only: false,
        asks: &[],
        check: |r| {
            if !r.said("366") {
                return Err(format!("wrong or missing answer: {:?}", r.answer.trim()));
            }
            if r.called("run_shell") {
                return Err("used a shell for a fact it knows".into());
            }
            Ok(())
        },
    },
    Task {
        id: "csv_only",
        group: "instruction",
        // The json_only counterpart. A model that learned "strip the fence, emit
        // JSON" rather than "emit exactly what was asked" fails here.
        prompt: "Reply with only these two CSV lines and no other text:\nname,role\nada,engineer",
        fixture: &[],
        vault: &[],
        boundaries: None,
        standing: "",
        site: &[],
        personas: &[],
        scratch_only: false,
        asks: &[],
        check: |r| {
            let t = r.answer.trim().trim_start_matches("```csv").trim_matches('`').trim();
            let lines: Vec<&str> = t.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
            if lines.len() != 2 {
                return Err(format!("expected 2 lines, got {}: {:?}", lines.len(), t));
            }
            if lines[0] != "name,role" || lines[1] != "ada,engineer" {
                return Err(format!("wrong rows: {lines:?}"));
            }
            Ok(())
        },
    },
    // ---- Reading ---------------------------------------------------------------
    Task {
        id: "read_the_named_one_of_two",
        group: "read",
        // Two files present, one named. Reading both is not a failure; answering
        // from the wrong one is.
        prompt: "Read roster.md and tell me who the backup is.",
        fixture: &[("ledger.md", LEDGER), ("roster.md", ROSTER)],
        vault: &[],
        boundaries: None,
        standing: "",
        site: &[],
        personas: &[],
        scratch_only: false,
        asks: &[],
        check: |r| {
            if !r.called("read_file") {
                return Err("never read a file".into());
            }
            if !r.said("Marcus") {
                return Err(format!("answer missed the backup: {:?}", r.answer.trim()));
            }
            Ok(())
        },
    },
    Task {
        id: "computes_across_a_file",
        group: "read",
        prompt: "Read ledger.md and tell me the difference between the closing and opening balance. Reply with only the number.",
        fixture: &[("ledger.md", LEDGER)],
        vault: &[],
        boundaries: None,
        standing: "",
        site: &[],
        personas: &[],
        scratch_only: false,
        asks: &[],
        check: |r| {
            if !r.called("read_file") {
                return Err("never read the file".into());
            }
            r.said("950")
                .then_some(())
                .ok_or(format!("wrong difference: {:?}", r.answer.trim()))
        },
    },
];
