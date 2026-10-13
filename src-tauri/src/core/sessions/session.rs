use super::*;

/// One connector's standing in a session: whether it is live here, and how far it
/// may go. `scope` is the source half of the same trust model `Root::writable` is
/// the folder half of — a source you may consult is not automatically a source you
/// may act through.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConnectionGrant {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_scope")]
    pub scope: String,
}

impl Default for ConnectionGrant {
    fn default() -> Self {
        ConnectionGrant { enabled: default_enabled(), scope: default_scope() }
    }
}

impl ConnectionGrant {
    pub fn writable(&self) -> bool {
        self.enabled && self.scope == SCOPE_WRITE
    }
}

/// The persisted session record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub session_id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub workspace: String,
    /// The persona this session was born from.
    pub agent: String,
    #[serde(default)]
    pub model: String,
    #[serde(default = "default_mode")]
    pub mode: String,
    #[serde(default)]
    pub created_at: f64,
    #[serde(default)]
    pub updated_at: f64,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub archived: bool,
    #[serde(default)]
    pub messages: Vec<Message>,
    /// Set when a platform mention spawned this session (Slack), not the user.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<String>,
    /// Set on a *sub-thread*: the session this one was split out of. One request
    /// containing several tasks becomes one child session per task, each working
    /// independently, and the sidebar nests them under their parent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    /// How many leading messages `/compact` folded into `compact_summary`. The
    /// transcript itself is never rewritten — the user keeps every message on
    /// screen — this only changes what the next run is *told*. Compacting is
    /// therefore reversible by clearing these two fields.
    #[serde(default)]
    pub compact_from: usize,
    /// Prose standing in for the messages before `compact_from`.
    #[serde(default)]
    pub compact_summary: String,
    /// When self-evolution last reviewed this session. Turns are counted from here,
    /// so one long-running conversation can be reviewed several times without
    /// re-judging material an earlier pass already saw. 0 = never reviewed.
    #[serde(default)]
    pub evolved_at: f64,
    /// This conversation's working folder, pinned the first time anything asked
    /// for it and never recomputed.
    ///
    /// Empty means not yet pinned. It has to be recorded rather than derived,
    /// because deriving it from the current settings is what made the Files card's
    /// promise — "existing conversations keep their current folder" — impossible to
    /// keep: every past conversation would move the moment someone changed where
    /// new ones go, orphaning the files they already contain.
    #[serde(default)]
    pub dir: String,
    /// When the profile learner last read this conversation. Kept separate from
    /// `evolved_at` because the two passes are gated separately and read different
    /// halves of the same transcript: evolution reads the whole exchange to improve
    /// a skill, this one reads only what the *user* said.
    #[serde(default)]
    pub profiled_at: f64,
    /// Per-session connector grants, keyed by connector id. An ABSENT entry means
    /// "inherit the persona default" — the map records only what the user settled
    /// here, so a persona's defaults can still move under an existing session.
    #[serde(default)]
    pub connections: BTreeMap<String, ConnectionGrant>,
}

pub(super) fn default_mode() -> String {
    "interactive".into()
}
