use crate::harness_eval::*;

pub(super) const BRAIN: &[Task] = &[
    // ---- Brain: does the right memory reach the answer -----------------------
    //
    // BRAIN-RESEARCH-2026 §3. Nineteen tasks graded the loop to four decimal places
    // and not one of them read the vault, so "the brain works" was a feeling rather
    // than a number. The four competencies below are MemoryAgentBench's; the fifth
    // is ours, and is the vault-side sibling of `does_not_invent_a_field`.
    Task {
        id: "recalls_a_stored_fact",
        group: "brain",
        prompt: "There is a deploy freeze on. What is the freeze override code? Check what you remember.",
        fixture: &[],
        vault: &[("github-4021.md", MEM_FREEZE)],
        boundaries: None,
        standing: "",
        site: &[],
        personas: &[],
        scratch_only: false,
        asks: &[],
        check: |r| {
            r.said_marker()
                .then_some(())
                .ok_or(format!("did not recall the override code: {:?}", r.answer.trim()))
        },
    },
    Task {
        id: "prefers_the_newer_fact",
        group: "brain",
        // Two memories, same subject, same wording, different dates and different
        // names. Nothing but the date separates them, and nothing in the retriever
        // reads the date — which is why this fails until supersession lands. It is
        // here to be red, and to go green when it is fixed.
        prompt: "Who is currently carrying the platform pager? Check what you remember.",
        fixture: &[],
        vault: &[("github-8801.md", MEM_ONCALL_OLD), ("github-9440.md", MEM_ONCALL_NEW)],
        boundaries: None,
        standing: "",
        site: &[],
        personas: &[],
        scratch_only: false,
        asks: &[],
        check: |r| {
            if !r.said_marker() {
                return Err(format!("did not name the current on-call: {:?}", r.answer.trim()));
            }
            // Naming both is not a pass. "It was Dana, now it's X" would be, but so
            // would "Dana or X", and the second is the failure this task exists for.
            // The narrow rule — the superseded name must not appear — is the one
            // that cannot be satisfied by hedging.
            if r.said("Dana") {
                return Err(format!("also offered the superseded name: {:?}", r.answer.trim()));
            }
            Ok(())
        },
    },
    Task {
        id: "joins_two_memories",
        group: "brain",
        prompt: "Where should I escalate a Project Perseus incident? Check what you remember.",
        fixture: &[],
        vault: &[("notion-311.md", MEM_PERSEUS), ("notion-312.md", MEM_ATLAS)],
        boundaries: None,
        standing: "",
        site: &[],
        personas: &[],
        scratch_only: false,
        asks: &[],
        check: |r| {
            // Neither memory answers alone. Only the first names Perseus, which is
            // all the question gives it to search on; only the second holds the
            // channel. The hop between them is the team name.
            r.said_marker()
                .then_some(())
                .ok_or(format!("did not join the two memories: {:?}", r.answer.trim()))
        },
    },
    Task {
        id: "does_not_invent_a_memory",
        group: "brain",
        // The vault has plenty to say about the deploy pipeline and nothing at all
        // about money. A confident number here is the failure that makes a memory
        // assistant worse than no memory assistant.
        prompt: "According to what you remember, what is the Q3 infrastructure budget?",
        fixture: &[],
        vault: &[("github-2200.md", MEM_PIPELINE)],
        boundaries: None,
        standing: "",
        site: &[],
        personas: &[],
        scratch_only: false,
        asks: &[],
        check: |r| {
            // Widened after the first run, where llama3.2 answered "I couldn't find
            // any relevant information about the Q3 infrastructure budget" — the
            // correct answer — and the check called it a failure. A checker that
            // fails an honest answer teaches the wrong lesson twice: it hides the
            // capability and it invites a fix to a defect that is not there.
            if !r.said_any(&[
                "not", "no ", "nothing", "doesn't", "does not", "don't", "unable", "isn't", "n/a", "couldn't",
                "could not", "cannot", "can't", "no record", "nowhere",
            ]) {
                return Err(format!("did not say the vault is silent on this: {:?}", r.answer.trim()));
            }
            // And it must not hedge its way to a figure anyway.
            if r.answer.contains('$') {
                return Err(format!("produced a figure the vault does not hold: {:?}", r.answer.trim()));
            }
            Ok(())
        },
    },
    Task {
        id: "applies_a_stated_preference",
        group: "brain",
        // Test-time learning: a rule the brain already holds has to govern an answer
        // about something else entirely. The rule is internal, so it arrives in the
        // opening brief — which makes this the task that would have caught Finding 0.
        prompt: "What are the biggest risks of deploying on a Friday afternoon?",
        fixture: &[],
        vault: &[("answer-format.md", MEM_FORMAT_RULE)],
        boundaries: None,
        standing: "",
        site: &[],
        personas: &[],
        scratch_only: false,
        asks: &[],
        check: |r| {
            // Any bullet marker counts, and the *number* is the assertion. Relaxed
            // after the first run: llama3.2 answered in `* ` bullets, and failing
            // that would have graded whether a model can emit a particular
            // character rather than whether a rule it was given governed the shape
            // of an answer about something else. The count is the substantive half
            // of the rule and the half that cannot be satisfied by accident.
            let bullets = r
                .answer
                .lines()
                .filter(|l| {
                    let l = l.trim_start();
                    l.starts_with("- ") || l.starts_with("* ") || l.starts_with("• ")
                })
                .count();
            if bullets != 3 {
                return Err(format!("the standing rule asks for exactly three bullets; got {bullets}: {:?}", r.answer.trim()));
            }
            Ok(())
        },
    },
];
