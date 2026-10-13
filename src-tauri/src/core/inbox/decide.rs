use super::*;

pub(super) fn now() -> f64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0)
}

/// What to do with a tool call that could have an effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// Run it now.
    Allow,
    /// Needs a human: interactive sessions prompt, unattended runs park an ask.
    Ask,
    /// Refuse (the persona's mode forbids acting).
    Deny,
}

/// Decide whether a tool call may proceed.
///
/// - `mode` is the persona's permission mode: `discuss`/`plan` never act,
///   `auto` acts freely, `interactive`/`custom` ask for anything consequential.
/// - `standing` is the task's pre-approved `tool target` list.
/// - `tainted` says this run has already read content from outside the machine.
/// - Read-class tools are always allowed.
///
/// # Taint
///
/// Everything above answers "is this *action* allowed?". None of it answers "who
/// asked for it?", and that gap is exploitable. A connector sync writes a stranger's
/// words into the vault; recall feeds them to the agent; the agent holds
/// `send_message`. An automation with a standing rule for `send_message
/// telegram:123` would therefore carry out an instruction found inside a synced
/// email, and this function would have said Allow — the tool and target match the
/// rule exactly, which is precisely what the attacker arranges.
///
/// So a tainted run loses its shortcuts for anything that can reach back out
/// ([`RiskClass::External`] and [`RiskClass::Exec`] — a shell can `curl` as easily
/// as a sender can send). Standing rules stop applying and `auto` stops
/// auto-allowing; the call becomes an `Ask`, which an attended session prompts for
/// and an unattended one parks in the inbox. `WriteLocal` is untouched: writing a
/// file on this machine doesn't leak anything, and blocking it would break ordinary
/// "read my inbox and draft a summary" work for no gain.
///
/// The user can still approve any single call. What they no longer do is approve it
/// once, in advance, for reasons that turn out to have been written by someone else.
pub fn decide(
    tool: &str,
    target: &str,
    risk: RiskClass,
    mode: &str,
    standing: &[String],
    tainted: bool,
) -> Decision {
    if risk == RiskClass::Read {
        return Decision::Allow;
    }
    // Modes that never act say no before anything else is considered.
    if matches!(mode, "discuss" | "plan") {
        return Decision::Deny;
    }
    // Tainted + can-reach-outside: a human decides this specific call.
    if tainted && matches!(risk, RiskClass::External | RiskClass::Exec) {
        return Decision::Ask;
    }
    // A standing rule pre-approves an exact tool+target (or the bare tool name).
    let entry = if target.is_empty() { tool.to_string() } else { format!("{tool} {target}") };
    if standing.iter().any(|r| r == &entry || r == tool) {
        return Decision::Allow;
    }
    match mode {
        "auto" => Decision::Allow,
        _ => Decision::Ask, // interactive | custom | anything unknown → fail safe
    }
}

/// A short id for something that needs one and has no natural key.
pub fn short_id() -> String {
    format!("{:x}", (now() * 1000.0) as u64)
}

/// Does this tool name belong to a connected app (an MCP tool) rather than one of
/// ours? Connector tools are named `<app>_<verb>`, and none of the built-in tools
/// take that shape — they're bare verbs like `run_shell` or `send_message`.
///
/// Used to decide whether carrying out an approved call needs a live connector
/// session opened for it, which is expensive enough not to do speculatively.
pub fn is_connector_tool(tool: &str) -> bool {
    const BUILT_IN: &[&str] = &[
        "send_message",
        "computer_use",
        "run_shell",
        "write_file",
        "edit_file",
        "read_file",
        "list_dir",
        "spawn_subagent",
        "connector_sync",
        "connector_search",
        "connector_changes",
        "web_fetch",
        "web_search",
        "knowledge_write",
        "recall_memory",
    ];
    // The browser's tools share a prefix the way an app's do, and are ours.
    !BUILT_IN.contains(&tool) && tool.contains('_') && !tool.starts_with("browser_")
}

/// A parked request for permission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ask {
    pub id: String,
    pub session_id: String,
    #[serde(default)]
    pub task_id: Option<String>,
    pub tool: String,
    #[serde(default)]
    pub target: String,
    pub summary: String,
    pub args: Value,
    pub created_at: f64,
    /// pending | approved | denied
    pub status: String,
    #[serde(default)]
    pub resolved_at: Option<f64>,
}

impl Ask {
    pub fn new(session_id: &str, task_id: Option<String>, tool: &str, target: &str, summary: &str, args: Value) -> Self {
        let n: u64 = rand::random();
        Ask {
            id: format!("ask-{:010x}", n & 0xff_ffff_ffff),
            session_id: session_id.to_string(),
            task_id,
            tool: tool.to_string(),
            target: target.to_string(),
            summary: summary.to_string(),
            args,
            created_at: now(),
            status: "pending".into(),
            resolved_at: None,
        }
    }
}

/// JSON-file-backed inbox of parked asks.
pub struct Inbox {
    pub(super) data_dir: PathBuf,
    pub(super) asks: Vec<Ask>,
}
