use super::*;

/// Expands to a long, boring, deterministic block of log lines.
///
/// A recall task needs output big enough to be truncated — tens of thousands of
/// characters — and pasting that into a source file would make this module
/// unreadable to save writing one loop. Deterministic so two runs of the same task
/// produce byte-identical fixtures, which is what lets the observation handle be
/// stable across repeats.
pub const FILLER_SLOT: &str = "{{FILLER}}";

/// How many filler lines one slot expands to. 1,400 lines of ~40 characters is
/// ~56k, comfortably past `TOOL_RESULT_CHAR_LIMIT` (20k) so the middle really is
/// cut rather than merely close to it.
pub(super) const FILLER_LINES: usize = 1_400;

pub(super) fn filler() -> String {
    (0..FILLER_LINES)
        .map(|i| format!("[info] step {i:05} nominal, nothing to see here\n"))
        .collect()
}

/// One measurable thing the harness should be able to do.
#[derive(Clone)]
pub struct Task {
    pub id: &'static str,
    /// What the task exercises, for the report's grouping.
    pub group: &'static str,
    pub prompt: &'static str,
    pub fixture: &'static [Fixture],
    /// Memories seeded into the instance's **vault** before the turn.
    ///
    /// A working folder cannot hold a brain. The vault, journal, knowledge pages
    /// and profile live in the app's data directory, not in the folder a task can
    /// write to, so a task that wants to grade recall has to seed the store the
    /// retriever actually reads. Each entry is a complete vault file, frontmatter
    /// and all, because the frontmatter *is* the fixture for half of these: the
    /// `date` is what `prefers_the_newer_fact` turns on and the `origin` is what
    /// decides whether SuperContext may quote it.
    ///
    /// Non-empty here is a hard requirement on the driver: the run is refused
    /// unless it is pointed at a scratch data directory. See `scratch_vault`.
    pub vault: &'static [Fixture],
    /// The boundaries in force for this task, as `boundaries.json` holds them.
    ///
    /// Same rule as `vault`, for the same reason: applying these replaces the
    /// boundaries of whatever app is listening, so a task that sets any runs only on a
    /// provably disposable instance. On one, every task is given its own — `None`
    /// resets to none — so a rule from one task never refuses the next.
    pub boundaries: Option<&'static str>,
    /// Standing instructions for this task, `{{MARKER}}` substituted. Empty is none, and
    /// is also applied, for the same reason as `boundaries`.
    pub standing: &'static str,
    /// True for a task that changes the app's own state without seeding anything — one
    /// that parks an approval, say. The seeded fields below imply this; this is for the
    /// tasks that would otherwise leave something behind in somebody's real app.
    pub scratch_only: bool,
    /// Assistants installed for this task, as manifest files — `{{MARKER}}` substituted,
    /// which is how a specialist comes to hold a fact nothing else on the machine has.
    ///
    /// Scratch-only and cleared between tasks, like `vault`: installing an assistant
    /// writes into the app's own data directory.
    pub personas: &'static [Fixture],
    /// Who the assistant under test may ask, for this task. Applied on every scratch run —
    /// an empty list clears the last task's grants rather than inheriting them.
    pub asks: &'static [&'static str],
    /// Pages the driver serves on a loopback port for this task, reached as `{{SITE}}` in
    /// the prompt by the assistant's own browser.
    ///
    /// Non-empty makes this a browser task, which changes the instance — the browser
    /// switched on, loopback allowed, the assistant given one, and this conversation's
    /// "Always allow" on the fixture host, which a person would otherwise press on the
    /// first card — so it runs only on a scratch instance, like `vault`.
    pub site: &'static [Page],
    /// Mechanical. `Ok(())` is a pass; the string is why it failed.
    pub check: fn(&TurnRecord) -> Result<(), String>,
}
