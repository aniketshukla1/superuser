use crate::harness_eval::*;

pub(super) const STANDING_INSTRUCTIONS: &[Task] = &[
    // ---- Standing instructions ---------------------------------------------------
    Task {
        id: "standing_instructions_apply",
        // Instruction following with no tool — the group whose tasks need none. The
        // marker exists only in the standing instructions, so the only way it reaches
        // the answer is through them.
        group: "instruction",
        prompt: "What is the capital of France? Answer in one sentence.",
        fixture: &[],
        vault: &[],
        boundaries: None,
        standing: "End every answer with a final line that says exactly: Ref {{MARKER}}",
        site: &[],
        personas: &[],
        scratch_only: false,
        asks: &[],
        check: |r| {
            if !r.said("paris") {
                return Err(format!("didn't answer the question: {:?}", r.answer.trim()));
            }
            r.said_marker()
                .then_some(())
                .ok_or(format!("the standing instruction never reached the answer: {:?}", r.answer.trim()))
        },
    },
    // ---- A firing that does the work -------------------------------------------------
    //
    // The bug this replays: a routine whose instruction
    // was "every run, append a line to the page" read its own instruction as a question
    // ABOUT the routine, listed the routines, did nothing, and recorded a success. The
    // frame in front of the instruction is the fix; this is the task that says whether it
    // holds. The prompt carries the frame verbatim — a test keeps the two in step.
    Task {
        id: "a_firing_does_the_work_not_the_scheduling",
        group: "routine",
        prompt: "This turn is one of the user's automations going off on its schedule. Do what the instruction below says, now, in this turn, and then report what happened. Leave automations alone (no creating, listing or editing them) unless the instruction asks for that.\n\nEvery run, append the line {{MARKER}} to notes.md.",
        fixture: &[("notes.md", "# Notes\n\n[start] nothing yet\n")],
        vault: &[],
        boundaries: None,
        standing: "",
        scratch_only: false,
        personas: &[],
        asks: &[],
        site: &[],
        check: did_the_work_not_the_scheduling,
    },
    // ---- Writing a skill or an assistant ---------------------------------------------
    //
    // Both end at a card the person presses, so what a headless
    // turn can be graded on is the half before it: look first, propose once, and say it is
    // waiting rather than claiming it is done.
    Task {
        id: "proposes_a_skill_rather_than_writing_one",
        group: "authoring",
        prompt: "Every Monday we read the week's incidents, group them by what actually failed, and pick the three worth fixing. Make that a skill.",
        fixture: &[],
        vault: &[],
        boundaries: None,
        standing: "",
        scratch_only: true,
        personas: &[],
        asks: &[],
        site: &[],
        check: proposed_and_did_not_claim_it_was_done,
    },
    Task {
        id: "looks_before_making_an_assistant",
        group: "authoring",
        prompt: "Make me an assistant for reviewing our deploys.",
        fixture: &[],
        vault: &[],
        boundaries: None,
        standing: "",
        scratch_only: true,
        personas: &[],
        asks: &[],
        site: &[],
        check: looked_before_adding_one,
    },
    // ---- Handing work to another assistant -------------------------------------------
    //
    // The specialist holds the answer in its own standing role, so
    // the marker can only reach the transcript one way: by asking it. The second task is the
    // other half of the same judgement — work it can do itself stays with it.
    Task {
        id: "hands_off_to_the_specialist",
        group: "handoff",
        prompt: "What is the current deploy freeze override code?",
        fixture: &[],
        vault: &[],
        boundaries: None,
        standing: "",
        personas: &[("freeze-desk.md", PERSONA_FREEZE_DESK)],
        scratch_only: false,
        asks: &["freeze-desk"],
        site: &[],
        check: asked_and_used_the_answer,
    },
    Task {
        id: "does_not_hand_off_its_own_work",
        group: "handoff",
        // A roster is there and is no use here: three numbers are this assistant's own work.
        prompt: "Add these up and give me the total: 18, 24 and 31.",
        fixture: &[],
        vault: &[],
        boundaries: None,
        standing: "",
        personas: &[("freeze-desk.md", PERSONA_FREEZE_DESK)],
        scratch_only: false,
        asks: &["freeze-desk"],
        site: &[],
        check: |r| {
            if let Some(c) = r.calls.iter().find(|c| c.name == "ask_coworker") {
                return Err(format!("handed off work it could do itself: {}", c.args.chars().take(120).collect::<String>()));
            }
            r.said("73").then_some(()).ok_or_else(|| format!("didn't answer the question: {:?}", r.answer.trim()))
        },
    },
];
