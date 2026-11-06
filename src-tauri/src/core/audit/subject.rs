use super::*;

pub(super) fn now() -> f64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0)
}

/// The current month's file. Older months are `audit-YYYY-MM.jsonl` beside it.
pub const FILE: &str = "audit.jsonl";

pub(super) const HEAD: &str = "audit.head";

/// One writer at a time, process-wide. The chain is only a chain if two appends
/// cannot both read the same "previous" hash.
pub(super) static WRITER: Mutex<()> = Mutex::new(());

/// Who or what started the run an action belongs to.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Initiator {
    /// `person` · `approval` · `schedule` · `watch` · `message` · `subagent` · `api` · `startup`
    pub kind: String,
    /// The schedule, watch, channel or parent session it came from, when there is one.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub id: String,
}

impl Initiator {
    pub fn new(kind: &str, id: &str) -> Self {
        Initiator { kind: kind.into(), id: id.into() }
    }

    /// What started a run, read off the session it runs in.
    ///
    /// Session ids are minted by the platform, never by a model, and every kind of
    /// run already has its own shape — which is what makes this safe to derive
    /// rather than to carry: a run cannot relabel itself as a person by choosing a
    /// name, because it never chooses one.
    pub fn from_session(session: &str) -> Self {
        // A subagent first: `abc-sub2` is a delegate of whatever `abc` was.
        if let Some((parent, n)) = session.rsplit_once("-sub") {
            if !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()) {
                return Initiator::new("subagent", parent);
            }
        }
        // `ask2-91f0` — one assistant asked another, and the digit is how far from the
        // person the work has travelled. Parsed here rather than imported, so the trail
        // does not depend on the tool that writes into it.
        if let Some(rest) = session.strip_prefix("ask") {
            let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if !digits.is_empty() && rest[digits.len()..].starts_with('-') {
                return Initiator::new("handoff", session);
            }
        }
        if let Some(run) = session.strip_prefix("__run__") {
            return Initiator::new("schedule", run);
        }
        if let Some(task) = session.strip_prefix("__task__") {
            return Initiator::new("schedule", task);
        }
        if session.starts_with("watch-") {
            return Initiator::new("watch", session);
        }
        for channel in ["slack-", "telegram-"] {
            if session.starts_with(channel) {
                return Initiator::new("message", channel.trim_end_matches('-'));
            }
        }
        // The stateless HTTP endpoint mints `run-<millis>`, and the scoreboard names
        // its own. Neither has a person watching the app.
        if session.starts_with("run-") || session.starts_with("eval-") || session.starts_with("ho-") {
            return Initiator::new("api", "");
        }
        Initiator::new("person", "")
    }

    /// True for the runs nobody is watching as they happen — the "Nobody watching"
    /// filter. A subagent is not one: it runs inside a turn someone is looking at.
    pub fn unwatched(&self) -> bool {
        matches!(self.kind.as_str(), "schedule" | "watch" | "message" | "api")
    }
}

/// The facts an action was judged on — the same facts a rule reads.
///
/// Stored so a candidate boundary can be replayed over history and decide exactly
/// what it would have decided live (see `core::boundaries::replay`). Every field is
/// written down by the rules in the module note, never copied raw from arguments.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Subject {
    pub tool: String,
    /// What the action does, rather than which tool did it. See `core::gateway::intent_of`.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub intent: String,
    /// `read` · `write_local` · `exec` · `external`
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub risk: String,
    /// What it touched, in words: a host, a path, a channel, a connected app.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub target: String,
    /// Verbatim, after [`scrub`]: a shell rule has to match the exact command
    /// that was typed.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub command: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub file: String,
    /// Scheme, host and path. Never the query — see [`clean_url`].
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub page: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub mcp_server: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub mcp_tool: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub mcp_effect: String,
    /// The length of text the action carried (a message, a typed value) — never the
    /// text itself.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chars: Option<u64>,
    /// The run had read content from outside this machine before this action.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub tainted: bool,
    /// The element a browser action was aimed at, as THE BROWSER'S OWN snapshot names it —
    /// never as the model described it, or "never click Submit" is evaded by calling the
    /// button something else.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub element_ref: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub element_role: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub element_name: String,
    /// An input's `type` — `password`, `email`.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub element_type: String,
    /// A key a browser action presses: `Enter` for a submit, or the key named.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub key: String,
}

/// What the boundaries decided about one action.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Decision {
    /// What the rules said.
    pub allowed: bool,
    /// `enforce` or `record`.
    pub mode: String,
    /// `deny` · `allow` · `default` · `floor`
    pub source: String,
    /// The expression that decided it; empty when nothing matched.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub rule: String,
    /// The name the user gave that rule, if any.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub rule_name: String,
    /// Whether the action went ahead. False only for a refusal in `enforce`.
    pub carried_out: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub reason: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Entry {
    pub at: f64,
    /// What happened: "sync" | "connect" | "send" | "exec" | "approve" | "deny" | "agent".
    /// For a version-2 row, the family of its `event` ("action", "policy", …).
    pub kind: String,
    /// Who did it — a connector id, persona id, or "user".
    pub actor: String,
    /// What it touched (a channel, host, connector, path).
    pub target: String,
    pub detail: String,
    /// False when the action failed.
    pub ok: bool,
    /// The typed event: `action.allowed`, `action.refused`, `action.failed`,
    /// `action.stopped`, `action.parked`, `action.declined`, `policy.loaded`,
    /// `policy.changed`, `turn.stalled`, `assistant.declined`, `tools.offered`,
    /// `audit.wiped`, `audit.retention` — or, on a legacy call, its `kind`.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub event: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub initiator: Option<Initiator>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub persona: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub session: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<Subject>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decision: Option<Decision>,
    /// The hash of the row before this one.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub prev: String,
    /// This row's own hash, over everything above it including `prev`.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub hash: String,
}
