use crate::harness_eval::*;

pub(super) const WRITING: &[Task] = &[
    // ---- Writing ---------------------------------------------------------------
    Task {
        id: "write_exact_contents",
        group: "write",
        prompt: "Create a file called notes.md whose entire contents are exactly: hello",
        fixture: &[],
        vault: &[],
        boundaries: None,
        standing: "",
        site: &[],
        personas: &[],
        scratch_only: false,
        asks: &[],
        check: |r| {
            let f = r.file("notes.md").ok_or("notes.md was not created")?;
            (f.trim() == "hello").then_some(()).ok_or(format!("contents were {:?}", f))
        },
    },
    Task {
        id: "edit_without_clobbering",
        group: "write",
        prompt: "In config.txt, change the timeout from 30 to 60. Leave everything else exactly as it is.",
        fixture: &[("config.txt", "retries = 5\ntimeout = 30\nverbose = true\n")],
        vault: &[],
        boundaries: None,
        standing: "",
        site: &[],
        personas: &[],
        scratch_only: false,
        asks: &[],
        check: |r| {
            let f = r.file("config.txt").ok_or("config.txt is gone")?;
            if !f.contains("timeout = 60") {
                return Err(format!("timeout not changed: {f:?}"));
            }
            // The other half, and the one a whole-file rewrite fails: everything that
            // was not asked about has to survive.
            for keep in ["retries = 5", "verbose = true"] {
                if !f.contains(keep) {
                    return Err(format!("lost {keep:?}: {f:?}"));
                }
            }
            Ok(())
        },
    },
    Task {
        id: "append_keeps_the_rest",
        group: "write",
        prompt: "Add a line saying four to the end of list.md. Keep the existing lines.",
        fixture: &[("list.md", "one\ntwo\nthree\n")],
        vault: &[],
        boundaries: None,
        standing: "",
        site: &[],
        personas: &[],
        scratch_only: false,
        asks: &[],
        check: |r| {
            let f = r.file("list.md").ok_or("list.md is gone")?.to_lowercase();
            for keep in ["one", "two", "three", "four"] {
                if !f.contains(keep) {
                    return Err(format!("missing {keep:?}: {f:?}"));
                }
            }
            Ok(())
        },
    },
    Task {
        id: "edit_the_right_occurrence",
        group: "write",
        prompt: "In app.conf, change the port on the metrics line to 9000. Do not touch the api line.",
        fixture: &[("app.conf", "api   host=localhost port=8080\nmetrics host=localhost port=8080\n")],
        vault: &[],
        boundaries: None,
        standing: "",
        site: &[],
        personas: &[],
        scratch_only: false,
        asks: &[],
        check: |r| {
            let f = r.file("app.conf").ok_or("app.conf is gone")?;
            if !f.contains("metrics host=localhost port=9000") {
                return Err(format!("metrics port not changed: {f:?}"));
            }
            // An ambiguous `old_str` is refused by `edit_file` rather than applied to
            // the first match, so the model has to disambiguate. This is the check
            // that says whether it did.
            f.contains("api   host=localhost port=8080")
                .then_some(())
                .ok_or(format!("changed the api line too: {f:?}"))
        },
    },
    // ---- Shell -----------------------------------------------------------------
    Task {
        id: "shell_reports_output",
        group: "shell",
        prompt: "Run the shell command: echo SU-42-OK . Tell me exactly what it printed.",
        fixture: &[],
        vault: &[],
        boundaries: None,
        standing: "",
        site: &[],
        personas: &[],
        scratch_only: false,
        asks: &[],
        check: |r| {
            if !r.called("run_shell") {
                return Err("never ran a shell command".into());
            }
            r.said("SU-42-OK").then_some(()).ok_or(format!("answer missed the output: {:?}", r.answer.trim()))
        },
    },
    Task {
        id: "counts_what_is_there",
        group: "shell",
        prompt: "How many .md files are in this folder? Answer with just the number.",
        fixture: HAYSTACK,
        vault: &[],
        boundaries: None,
        standing: "",
        site: &[],
        personas: &[],
        scratch_only: false,
        asks: &[],
        check: |r| {
            if !r.called("list_dir") && !r.called("run_shell") && !r.called("grep") {
                return Err("answered without looking".into());
            }
            r.said("5").then_some(()).ok_or(format!("wrong count: {:?}", r.answer.trim()))
        },
    },
];
