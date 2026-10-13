//! Assistants a handoff task needs

use super::*;

//
// Each holds one fact in its own standing role and nothing else does — not the working
// folder, not the vault, not this file, since the marker is substituted per run. So an
// answer carrying it can only have come from asking the assistant that holds it.

pub(super) const PERSONA_FREEZE_DESK: &str = r#"---
id: freeze-desk
name: Freeze Desk
tagline: Keeps the deploy freeze and its override code
family: knowledge
kind: assistant
tools: []
default_permission_mode: interactive
---

You are the Freeze Desk. You keep one thing: the deploy freeze and the code that overrides it.

The current freeze override code is {{MARKER}}. When anyone asks for it, give it plainly and say
nothing else about it. If you are asked anything else, say that the freeze is all you keep.
"#;

pub(super) const PERSONA_BADGE_DESK: &str = r#"---
id: badge-desk
name: Badge Desk
tagline: Keeps the door codes
family: knowledge
kind: assistant
tools: []
default_permission_mode: interactive
---

You are the Badge Desk. You keep the building's door codes and nothing else.

The east door code is {{MARKER}}. Give it plainly when asked. If you are asked about anything other
than doors and badges, say that is not yours.
"#;

pub(super) const PERSONA_KITCHEN_ROTA: &str = r#"---
id: kitchen-rota
name: Kitchen Rota
tagline: Keeps the kitchen rota
family: knowledge
kind: assistant
tools: []
default_permission_mode: interactive
---

You are the Kitchen Rota. You keep who is on the kitchen rota each week and nothing else.

This week it is Priya, then Marcus, then Lena. You do not know any door codes, badge codes or
building access of any kind; if you are asked for one, say plainly that you don't keep those.
"#;

/// The "never submit a form in the browser" preset, as boundaries JSON. Its rule is the
/// preset's own, character for character — a test holds the two together.
pub(super) const BOUNDARY_NO_SUBMIT: &str = r#"{"mode":"enforce","deny":[{"name":"Never submit a form in the browser","preset":"never-submit-in-browser","when":"(intent == \"activate\" && (contains(element.name, \"submit\") || contains(element.name, \"send\") || contains(element.name, \"place order\") || contains(element.name, \"pay\") || contains(element.name, \"confirm\"))) || (tool.name in [\"browser_key\", \"browser_type\"] && key == \"Enter\")"}],"allow":["true"]}"#;

pub(super) const ALPHA: &str = "# Alpha\n\nThe Perseus launch date is 3 October 2027.\nThe code name is BLUEJAY.\n";

pub(super) const BETA: &str = "# Beta\n\nThe Perseus review is scheduled for 5 November 2027.\n";

/// The fixture for the search tasks: one file in five holds the marker.
///
/// `{{MARKER}}` is substituted by the driver with a token generated fresh per run,
/// and that indirection is the whole point. The marker used to be a literal nonsense
/// word, on the reasoning that no model could know it from training. True, and not
/// enough: the literal then sat in **this source file**, inside a repository whose
/// shell the agent can legitimately reach — `run_shell` is cwd-confined to the
/// workspace but is a real shell, by design, gated by approvals rather than by a
/// jail. `gemma4:12b` duly searched wider, found the marker in `harness_eval.rs`,
/// and answered with *that* filename. The task had a findable wrong answer.
///
/// Guarding against the model's memory and not against the filesystem is half a
/// guard. A per-run token cannot be anywhere but the fixture.
pub(super) const HAYSTACK: &[Fixture] = &[
    ("one.md", "Notes about deployment and rollback plans.\n"),
    ("two.md", "Meeting notes: budget, hiring, the usual.\n"),
    ("three.md", "Rollout checklist. Nothing unusual here.\n"),
    ("four.md", "The archive marker is {{MARKER}} and it belongs to the vault.\n"),
    ("five.md", "Postmortem notes from the incident last spring.\n"),
];

/// The fixture for the recall task: one very long log with the answer buried in
/// the middle, and one short file that gives the model a reason to take a second
/// step before it answers.
///
/// The second file is what makes this measure anything. `relieve_pressure` only
/// rewrites results from *earlier* steps — the result of the call the model just
/// made is protected — so a single read is never truncated and never needs
/// recalling. Reading the short file after the long one is what pushes the long
/// result behind the protection boundary.
///
/// Honest about its own limit: the model chooses the order. Reading `owner.md`
/// first and `build.log` second leaves the long result whole and the task passes
/// without recall ever being exercised. This grades the outcome — can the model
/// still answer from output that has been cut — and not the mechanism.
pub(super) const RECALL_LOG: &[Fixture] = &[
    (
        "build.log",
        "[info] build started\n{{FILLER}}[audit] ARCHIVE-KEY {{MARKER}} retired\n{{FILLER}}[info] build finished\n",
    ),
    ("owner.md", "# Owner\n\nThis build is owned by Priya Raman.\n"),
];
