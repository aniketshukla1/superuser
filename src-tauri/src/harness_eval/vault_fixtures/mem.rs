
//
// Complete vault files, frontmatter and all, because the frontmatter is half the
// fixture: `date` is what a conflict turns on and `origin` is what decides whether
// SuperContext may quote the body or may only count it.
//
// `origin: external` on all but one, deliberately. That is what a real vault looks
// like — the measured one was 125 memories and 0 internal — and it is the path that
// actually exercises retrieval: an external memory reaches the model only if it
// goes and calls `recall_memory`, where an internal one is handed to it in the
// opening brief. Grading recall on inlined memories would grade the brief.

/// One memory holding a fact nothing else in the vault knows.
pub(in crate::harness_eval) const MEM_FREEZE: &str = "---\ntitle: \"Deploy freeze window\"\ncategory: \"Engineering\"\ntags: ['deploy', 'freeze']\ndate: \"2026-08-14T09:12:00+00:00\"\nsummary: \"The open deploy freeze and its override code\"\nweight: 1.000\nreinforce_count: 0\nlast_interaction: \"2026-08-14T09:12:00+00:00\"\norigin: \"external\"\n---\n\nThe platform team has opened a deploy freeze for the quarter. The freeze override\ncode is {{MARKER}} and it must be quoted on any change request raised while the\nfreeze is in force.\n";

/// The older half of a contradiction. Same subject, same words, earlier date.
pub(in crate::harness_eval) const MEM_ONCALL_OLD: &str = "---\ntitle: \"Platform on-call rotation\"\ncategory: \"Engineering\"\ntags: ['oncall', 'platform']\ndate: \"2026-01-05T11:00:00+00:00\"\nsummary: \"Who carries the platform pager\"\nweight: 1.000\nreinforce_count: 0\nlast_interaction: \"2026-08-01T11:00:00+00:00\"\norigin: \"external\"\n---\n\nThe platform on-call rotation has been handed over. Dana Okoye carries the\nplatform pager from now on.\n";

/// The newer half. Nothing but the date says so.
pub(in crate::harness_eval) const MEM_ONCALL_NEW: &str = "---\ntitle: \"Platform on-call rotation\"\ncategory: \"Engineering\"\ntags: ['oncall', 'platform']\ndate: \"2026-08-20T11:00:00+00:00\"\nsummary: \"Who carries the platform pager\"\nweight: 1.000\nreinforce_count: 0\nlast_interaction: \"2026-08-01T11:00:00+00:00\"\norigin: \"external\"\n---\n\nThe platform on-call rotation has been handed over. {{MARKER}} carries the\nplatform pager from now on.\n";

/// Half of a two-hop answer: the project's owning team.
pub(in crate::harness_eval) const MEM_PERSEUS: &str = "---\ntitle: \"Project Perseus ownership\"\ncategory: \"Engineering\"\ntags: ['perseus']\ndate: \"2026-06-02T08:30:00+00:00\"\nsummary: \"Which team owns Perseus\"\nweight: 1.000\nreinforce_count: 0\nlast_interaction: \"2026-08-01T08:30:00+00:00\"\norigin: \"external\"\n---\n\nProject Perseus was transferred to the Atlas team at the start of the quarter.\nThey own the service, the runbook and the pager for it.\n";

/// The other half: that team's escalation channel. Neither memory answers alone.
pub(in crate::harness_eval) const MEM_ATLAS: &str = "---\ntitle: \"Atlas team escalation\"\ncategory: \"Engineering\"\ntags: ['atlas', 'escalation']\ndate: \"2026-06-11T16:45:00+00:00\"\nsummary: \"How to escalate to the Atlas team\"\nweight: 1.000\nreinforce_count: 0\nlast_interaction: \"2026-08-01T16:45:00+00:00\"\norigin: \"external\"\n---\n\nEscalations for anything the Atlas team owns go to the #{{MARKER}} channel.\nDo not page individuals directly outside business hours.\n";

/// A vault that is *related* to the question and does not answer it. The vault-side
/// sibling of `does_not_invent_a_field`: the same honesty property, one store over.
pub(in crate::harness_eval) const MEM_PIPELINE: &str = "---\ntitle: \"Deploy pipeline notes\"\ncategory: \"Engineering\"\ntags: ['deploy', 'pipeline']\ndate: \"2026-07-19T13:05:00+00:00\"\nsummary: \"How the deploy pipeline is wired\"\nweight: 1.000\nreinforce_count: 0\nlast_interaction: \"2026-08-01T13:05:00+00:00\"\norigin: \"external\"\n---\n\nThe deploy pipeline builds on merge, runs the integration suite, and promotes to\nstaging automatically. Production promotion is manual and needs two approvals.\nNothing here records any budget, spend or cost figure.\n";

/// The one internal memory: a standing instruction, in the user's own words.
///
/// Internal so it arrives in the opening brief rather than behind `recall_memory` —
/// which is the point. This grades whether a rule the brain already holds actually
/// governs the answer, and it is the task that would have caught Finding 0, since a
/// starved inlined tier makes it unreachable.
pub(in crate::harness_eval) const MEM_FORMAT_RULE: &str = "---\ntitle: \"How I want answers formatted\"\ncategory: \"Preferences\"\ntags: ['format', 'answers', 'style']\ndate: \"2026-05-30T07:00:00+00:00\"\nsummary: \"Standing rule for how answers should be shaped\"\nweight: 1.000\nreinforce_count: 0\nlast_interaction: \"2026-08-01T07:00:00+00:00\"\norigin: \"internal\"\n---\n\nStanding rule for answers: always reply as exactly three bullet points and nothing\nelse. Every bullet starts with \"- \". No preamble, no closing paragraph.\n";

/// Held-out: a fact in the vault and nowhere else.
pub(in crate::harness_eval) const MEM_BADGE: &str = "---\ntitle: \"Office badge system\"\ncategory: \"Operations\"\ntags: ['badge', 'office']\ndate: \"2026-08-02T10:00:00+00:00\"\nsummary: \"The badge reader replacement\"\nweight: 1.000\nreinforce_count: 0\nlast_interaction: \"2026-08-02T10:00:00+00:00\"\norigin: \"external\"\n---\n\nThe badge readers were replaced in August. The temporary visitor door code is\n{{MARKER}} and it expires at the end of the quarter.\n";

/// Held-out contradiction, older half.
pub(in crate::harness_eval) const MEM_VENDOR_OLD: &str = "---\ntitle: \"Storage vendor\"\ncategory: \"Operations\"\ntags: ['storage', 'vendor']\ndate: \"2025-11-12T09:00:00+00:00\"\nsummary: \"Who supplies the storage tier\"\nweight: 1.000\nreinforce_count: 0\nlast_interaction: \"2026-08-01T09:00:00+00:00\"\norigin: \"external\"\n---\n\nThe storage tier is supplied by Northwind Systems under the current contract.\n";

/// Held-out contradiction, newer half.
pub(in crate::harness_eval) const MEM_VENDOR_NEW: &str = "---\ntitle: \"Storage vendor\"\ncategory: \"Operations\"\ntags: ['storage', 'vendor']\ndate: \"2026-07-28T09:00:00+00:00\"\nsummary: \"Who supplies the storage tier\"\nweight: 1.000\nreinforce_count: 0\nlast_interaction: \"2026-08-01T09:00:00+00:00\"\norigin: \"external\"\n---\n\nThe storage tier is supplied by {{MARKER}} under the current contract.\n";
