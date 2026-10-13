use super::*;

/// The session context an agent run executes against.
pub struct ToolCtx<'a> {
    pub root: PathBuf,
    pub memory: &'a crate::core::memory::MemoryEngine,
    pub settings: crate::core::settings::Settings,
    pub todo: Mutex<Vec<TodoItem>>,
    /// Where the app's state lives (inbox, stores).
    pub data_dir: PathBuf,
    /// This run's session id — parked asks are attributed to it.
    pub session_id: String,
    /// The persona driving this run. A subagent inherits it (same model, same
    /// workspace) while running under its own type's prompt and toolset.
    pub persona: String,
    /// The automation this run belongs to, if it's an unattended run.
    pub task_id: Option<String>,
    /// The persona's permission mode: discuss | plan | interactive | custom | auto.
    pub mode: String,
    /// Pre-approved `tool target` entries from the automation.
    pub standing: Vec<String>,
    /// True when nobody is watching (a scheduled run) — asks are parked, never assumed.
    pub unattended: bool,
    /// Set once this run has read content that came from outside this machine.
    ///
    /// Sticky by design. A run that has read a stranger's words cannot become clean
    /// again by reading something else afterwards — the words are still in the
    /// context, still able to motivate the next tool call. See
    /// [`crate::core::inbox::decide`] for what it costs the run.
    pub tainted: AtomicBool,
    /// Consecutive locator misses with no look in between — see `gates::groping`.
    /// A run-scoped counter rather than a per-signature one, because the loop this
    /// catches never repeats a signature: it guesses a *different* wrong path each
    /// time, which is exactly why `repetition` never fires on it.
    pub gropes: AtomicU32,
    /// Where this run's step events go, when anyone is attached.
    ///
    /// Only `ask_user` needs it, and it needs it for a structural reason: a question
    /// has to reach the user *before* the tool returns, because the tool's return
    /// value IS the answer. Every other tool can report after the fact.
    pub events: Option<EventSink>,
    /// What the user actually said this turn.
    ///
    /// The gates read it to tell an invented argument from a quoted one — see
    /// [`crate::core::gates::looks_invented`]. A value the user typed is theirs,
    /// however much it looks like a placeholder.
    pub prompt: String,
    /// What this run has genuinely read: paths opened, pages fetched.
    ///
    /// The grounding gate checks a chart's stated source against it. Kept per-run
    /// rather than per-session on purpose — a file read an hour ago in another turn
    /// is not in this turn's context, so charting from it is still guessing.
    pub seen: Mutex<Vec<String>>,
    /// The model this run is on. A hop runs on it too — a delegated run that quietly
    /// picked the app's default would make a sweep compare two models at once, and would
    /// send a local-only user's work to whatever the default happens to be.
    pub model: String,
    /// The coworkers this turn has already asked, by ask key.
    ///
    /// One list answers both questions a handoff asks — how many has this turn asked, and
    /// has it asked exactly this already — because they are the same question about the
    /// same turn, and keeping two would let them disagree.
    pub asked: Mutex<Vec<String>>,
    /// Failing call signatures and how many times each has failed, for the
    /// repetition gate.
    pub calls: Mutex<std::collections::HashMap<String, u32>>,
    /// Succeeding call signatures, the hash of what each last returned, and how many
    /// times in a row it has returned exactly that — for the stall gate. See
    /// [`ToolCtx::note_result`] for why this is not the same question as `calls`.
    pub stalls: Mutex<std::collections::HashMap<String, (u64, u32)>>,
    /// Links this run came across in tool results — search hits, links on a fetched
    /// page, URLs in a connected app's reply.
    ///
    /// Kept apart from `seen` deliberately. `seen` answers "did this run READ it",
    /// which is what a chart's source has to satisfy; this answers the weaker "does
    /// this run have any evidence it exists", which is the right bar for a citation.
    /// Mixing them would let a link found on a page count as having read that link,
    /// and would let one link-heavy page flood the list the chart gate reports.
    pub urls: Mutex<Vec<String>>,
    /// The auto-approve reviewer's circuit breaker for this turn.
    ///
    /// A streak, not a total: any non-deny resets it. A cumulative count would let
    /// one over-strict pair early in a long agentic turn silently downgrade the
    /// whole rest of it to hand-approval, which reads as the feature switching
    /// itself off for no reason.
    pub reviewer: crate::core::reviewer::Breaker,
    /// What each security check actually did — the Coverage ledger.
    ///
    /// Run-scoped, like `seen` and `urls`, and for the same reason: it describes
    /// THIS piece of work. `check_tool` writes to it the moment it finds a scanner
    /// missing, so a check cannot vanish between that discovery and the report.
    pub coverage: Mutex<Vec<crate::core::toolchain::Entry>>,
    /// The charts this run drew, kept so they survive the turn.
    ///
    /// The transcript draws a chart from the tool call that made it, and the store
    /// keeps only the final answer — so on reload the chart vanished from the
    /// conversation while the assistant's own text still said "as shown above". The
    /// specs ride out on the run's result and are persisted beside the todos.
    pub charts: Mutex<Vec<Value>>,
    /// What started this run, when the session id does not already say. `None` reads
    /// it off the session (`audit::Initiator::from_session`); the one case that
    /// cannot is an Inbox approval replayed against the session that parked it.
    pub initiator: Option<crate::core::audit::Initiator>,
}

/// Where a run's step events go — the session socket's end of the turn.
pub type EventSink = tokio::sync::mpsc::UnboundedSender<Value>;
