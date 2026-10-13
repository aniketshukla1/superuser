use super::super::*;

// ===========================================================================
//  The held-out split
// ===========================================================================

pub(in crate::harness_eval) const LEDGER: &str = "# Ledger\n\nOpening balance: 4200\nClosing balance: 5150\n";

pub(in crate::harness_eval) const ROSTER: &str = "# Roster\n\nOn call: Priya\nBackup: Marcus\n";

pub(in crate::harness_eval) const RELEASE: &str = "title: Perseus\nstatus: draft\nowner: platform\n";

/// Two columns and no phone number. Digit-free on purpose: it makes fabrication
/// mechanically detectable rather than a judgement about wording.
pub(in crate::harness_eval) const CONTACTS: &str = "name,email\nlena,lena@example.com\nomar,omar@example.com\n";

/// The fixture for the held-out search task. Same shape as [`HAYSTACK`] and
/// deliberately not the same files: a set that shares filenames with the feedback
/// set is a set the harness can have been tuned against by accident.
pub(in crate::harness_eval) const THICKET: &[Fixture] = &[
    ("intake.md", "Intake queue notes. Nothing filed this week.\n"),
    ("vendors.md", "Vendor contacts and renewal dates.\n"),
    ("runbook.md", "Restart procedure, then page the on-call.\n"),
    ("audit.md", "Standard audit trail, no exceptions raised.\n"),
    ("shipping.md", "The consignment token is {{MARKER}} and it routes to the depot.\n"),
    ("budget.md", "Quarterly figures, unchanged from the last revision.\n"),
];

/// Tasks withheld from development — the generalization check.
///
/// HarnessDev (arXiv:2609.01437, ByteDance Seed et al., 2026) makes withholding the
/// central discipline of its protocol: a creator develops against a small public
/// feedback set and is scored on hidden tasks it never sees, because a harness tuned
/// against the set that grades it stops measuring capability and starts measuring fit
/// to nineteen prompts.
///
/// This repository earned that lesson without naming it. Phases 4, 5 and 6 were each
/// graded against [`TASKS`] and then fixed until [`TASKS`] improved — `add48f3` grades
/// Phase 5 with the scoreboard and records that it fails its own criterion. That is
/// the right loop for finding a bug and the wrong one for claiming a gain: after three
/// rounds of it, [`TASKS`] is partly a training set and its totals overstate what a
/// model would score on work it had not been fitted to.
///
/// Same seven groups, same mechanical checks, different surface. A harness that passes
/// [`TASKS`] by having been tuned to it shows the gap here.
///
/// **The rule this file cannot enforce, which is what makes it worth writing down.**
/// These tasks run at phase boundaries and their failures are *reported*, never used
/// to choose a fix. The moment a change is made because a held-out task failed, this
/// stops being held out and a new split is owed. `HARNESS-BASELINE` records each run;
/// a run not recorded there did not happen.
///
/// **Authorship.** Written by hand, and deliberately not by the model the Evolution
/// stage points at this harness. A creator that authored its own hidden set has not
/// been held out from anything.
/// The held-out counterpart to [`RECALL_LOG`]. Same shape, nothing shared: a
/// different long file, a different buried fact, a different second file.
pub(in crate::harness_eval) const RECALL_TRACE: &[Fixture] = &[
    (
        "trace.txt",
        "[trace] session opened\n{{FILLER}}[trace] RELEASE-TAG {{MARKER}} promoted\n{{FILLER}}[trace] session closed\n",
    ),
    ("approver.md", "# Approval\n\nSigned off by Devika Menon.\n"),
];
