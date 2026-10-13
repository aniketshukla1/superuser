use crate::harness_eval::*;

pub(super) const INSTRUCTION_FOLLOWING: &[Task] = &[
    // ---- Instruction following: no tools required -----------------------------
    Task {
        id: "one_word",
        group: "instruction",
        prompt: "Reply with exactly one word: pong",
        fixture: &[],
        vault: &[],
        boundaries: None,
        standing: "",
        site: &[],
        personas: &[],
        scratch_only: false,
        asks: &[],
        check: |r| {
            let a = r.answer.trim().trim_matches('.').to_lowercase();
            (a == "pong").then_some(()).ok_or(format!("answered {:?}", r.answer.trim()))
        },
    },
    Task {
        id: "arithmetic_no_tools",
        group: "instruction",
        prompt: "What is 17 times 23? Reply with only the number and nothing else.",
        fixture: &[],
        vault: &[],
        boundaries: None,
        standing: "",
        site: &[],
        personas: &[],
        scratch_only: false,
        asks: &[],
        check: |r| {
            if !r.said("391") {
                return Err(format!("wrong or missing answer: {:?}", r.answer.trim()));
            }
            // A model reaching for a shell to multiply two two-digit numbers is
            // burning a step and a round trip on something it can do.
            if r.called("run_shell") {
                return Err("used a shell for mental arithmetic".into());
            }
            Ok(())
        },
    },
    Task {
        id: "json_only",
        group: "instruction",
        prompt: "Reply with only this JSON object and no other text: {\"ok\": true}",
        fixture: &[],
        vault: &[],
        boundaries: None,
        standing: "",
        site: &[],
        personas: &[],
        scratch_only: false,
        asks: &[],
        check: |r| {
            let t = r.answer.trim().trim_start_matches("```json").trim_matches('`').trim();
            let v: serde_json::Value =
                serde_json::from_str(t).map_err(|e| format!("not JSON ({e}): {:?}", r.answer.trim()))?;
            (v["ok"] == true).then_some(()).ok_or(format!("wrong JSON: {v}"))
        },
    },
    // ---- Reading ---------------------------------------------------------------
    Task {
        id: "read_named_file",
        group: "read",
        prompt: "Read the file alpha.md and tell me the code name it gives.",
        fixture: &[("alpha.md", ALPHA)],
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
            r.said("BLUEJAY").then_some(()).ok_or(format!("answer missed it: {:?}", r.answer.trim()))
        },
    },
    Task {
        id: "compare_two_files",
        group: "read",
        prompt: "alpha.md and beta.md each contain a date. Which file has the earlier one? Answer with the filename.",
        fixture: &[("alpha.md", ALPHA), ("beta.md", BETA)],
        vault: &[],
        boundaries: None,
        standing: "",
        site: &[],
        personas: &[],
        scratch_only: false,
        asks: &[],
        check: |r| {
            if r.said("beta") && !r.said("alpha") {
                return Err("picked the later date".into());
            }
            r.said("alpha").then_some(()).ok_or(format!("no filename in {:?}", r.answer.trim()))
        },
    },
];
