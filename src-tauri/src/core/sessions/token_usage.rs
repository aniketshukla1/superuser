use super::*;

/// Does this folder hold anything but the session's own record?
///
/// The record (`session.json`) lives in the working folder, so its presence says
/// nothing about whether the conversation has produced anything.
pub(super) fn has_deliverables(dir: &Path) -> bool {
    let Ok(rd) = fs::read_dir(dir) else { return false };
    rd.flatten().any(|e| e.file_name() != std::ffi::OsStr::new("session.json"))
}

/// Expand a leading `~` — the Files card's placeholder is `~/Superuser`, and a
/// literal directory called "~" in the app's data folder is nobody's intent.
pub(super) fn shellexpand_home(p: &str) -> String {
    match p.strip_prefix("~/") {
        Some(rest) => dirs::home_dir().map(|h| h.join(rest).to_string_lossy().to_string()).unwrap_or_else(|| p.to_string()),
        None if p == "~" => dirs::home_dir().map(|h| h.to_string_lossy().to_string()).unwrap_or_else(|| p.to_string()),
        _ => p.to_string(),
    }
}

pub(super) fn now() -> f64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0)
}

/// ISO-8601 in UTC — the shape the GUI's `updated_at` expects.
pub(super) fn iso(ts: f64) -> String {
    chrono::DateTime::from_timestamp(ts as i64, 0)
        .unwrap_or_else(chrono::Utc::now)
        .to_rfc3339()
}

/// One turn in the transcript. `items` carries the structured record of what the
/// assistant did (tool calls, plans) so a reload renders the same thing the live
/// socket showed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String, // "user" | "assistant" | "system"
    pub content: String,
    #[serde(default)]
    pub at: f64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<Value>,
    /// What this turn cost, when the provider said. Absent on user messages, on providers
    /// that report nothing, and on every message written before this existed — so anything
    /// reading it must treat missing as "unknown", never as zero.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<TokenUsage>,
    /// This turn's native message traffic — tool calls and their results, in the
    /// wire shape of the model that produced them.
    ///
    /// Deliberately not folded into `items`, which the plan assumed was already this:
    /// `items` is the UI's payload (todos, charts, spawned threads) and is read by the
    /// transcript renderer. Mixing a provider's wire format into it would put one
    /// concept in two places and hand the front end blocks it has no way to draw.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wire: Option<crate::core::transcript::TurnWire>,
    /// Who answered, when it wasn't this conversation's own assistant: the person opened
    /// the message with `@id` (`coworker::mentioned`). Absent on everything else.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub persona: Option<String>,
}

/// Tokens for one turn, as the provider reported them.
///
/// This was input and output only, and the reason given was a good one: providers
/// disagree about everything else — Anthropic bills cache reads and writes
/// separately, OpenAI folds cached input into `prompt_tokens`, Ollama has no cache
/// concept at all — so summing anything finer across providers adds up things that
/// are not the same.
///
/// The cache fields are here anyway, because the objection turns out to argue for
/// them. Anthropic is the provider that costs real money, and cache behaviour is the
/// single largest cost lever in HARNESS-PLAN (§2.7, a projected 60–80% cut). Folding
/// its cache counters into `input` would make that work unmeasurable in exactly the
/// place it matters: a cached step and an uncached one would report the same number.
///
/// The cross-provider objection is answered by the defaults rather than by omission.
/// A provider that reports no cache counters leaves them zero and contributes nothing
/// to a sum, which is the truth — it did no caching we were billed for. What must not
/// happen is the reverse: reading a zero as "the cache missed" when the provider
/// simply has no cache. That is what `cache_hit_rate` returning `Option` is for.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Which model this turn ran on.
    ///
    /// The GUI groups usage `byModel` and has always rendered a per-model table —
    /// with an "Unknown model" branch for rows that carry no id. Every row took that
    /// branch, because the sidecar the server sends is this struct and it had no
    /// model in it. A breakdown by model where the model is never known is a table
    /// with one row called "Unknown".
    ///
    /// Stored per MESSAGE rather than read from the session, because a conversation
    /// can change model mid-way and the point of the breakdown is to show that.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Fresh input — tokens the provider actually read this step. On Anthropic this
    /// EXCLUDES anything served from or written to the cache; those are separate
    /// counters, which is the whole reason they are separate fields here.
    #[serde(default)]
    pub input: u64,
    #[serde(default)]
    pub output: u64,
    /// Input served from the prompt cache, billed at 0.1×. The number that says
    /// whether caching is working at all.
    #[serde(default)]
    pub cache_read: u64,
    /// Input written into the cache, billed at 1.25×. Paid once so `cache_read` can
    /// be cheap afterwards — a turn where this is large and `cache_read` stays zero
    /// is paying the premium and getting nothing back.
    #[serde(default)]
    pub cache_write: u64,
}

impl TokenUsage {
    /// Every token the provider processed, however it was billed. Cache reads and
    /// writes are input the model saw; leaving them out would report a fully-cached
    /// turn as nearly free of tokens, which is the opposite of what happened.
    pub fn total(&self) -> u64 {
        self.input + self.output + self.cache_read + self.cache_write
    }
    /// Input in all its forms, for cost and cache-rate arithmetic.
    pub fn input_all(&self) -> u64 {
        self.input + self.cache_read + self.cache_write
    }
    /// What fraction of this step's input came from the cache.
    ///
    /// `None` when the provider reported no cache activity at all — which is the
    /// normal state for Ollama and the local runners, since they have no prompt
    /// cache to report. Rendering those as "0% hit rate" would accuse a provider of
    /// missing a cache it does not have, and would put a permanent zero on the
    /// dashboard that no amount of Phase 3 work could move.
    ///
    /// A *cold* cache is different and does return `Some(0.0)`: tokens were written
    /// and none read back yet. That zero is real, and it is supposed to climb.
    pub fn cache_hit_rate(&self) -> Option<f64> {
        let all = self.input_all();
        let reported_cache = self.cache_read > 0 || self.cache_write > 0;
        (all > 0 && reported_cache).then(|| self.cache_read as f64 / all as f64)
    }
    /// `Message::with_usage` drops empty usage so "the provider said nothing" cannot
    /// be stored as zeros. That makes this load-bearing: omit the cache fields here
    /// and a turn served entirely from cache — input 0, cache_read 15,000 — reads as
    /// empty and is thrown away, hiding exactly the turns the cache work is for.
    pub fn is_empty(&self) -> bool {
        self.total() == 0
    }
}

impl Message {
    pub fn new(role: &str, content: impl Into<String>) -> Self {
        Message { role: role.into(), content: content.into(), at: now(), items: Vec::new(), usage: None, wire: None, persona: None }
    }

    /// The same message with the wire transcript stripped, for anything sent to a
    /// client.
    ///
    /// `wire` exists for the *next model*, not for the reader: it is provider-shaped
    /// tool traffic the transcript renderer has no way to draw, capped at 24 KB a
    /// turn. Shipping it to the webview on every session open would add hundreds of
    /// kilobytes to a payload nothing reads.
    pub fn without_wire(&self) -> Self {
        Message { wire: None, ..self.clone() }
    }

    /// Attach what the turn cost. Empty usage is dropped rather than stored as zeros —
    /// "the provider said nothing" and "the turn was free" must not look alike.
    pub fn with_usage(mut self, usage: Option<TokenUsage>) -> Self {
        self.usage = usage.filter(|u| !u.is_empty());
        self
    }
}

pub const SCOPE_READ: &str = "read";

pub const SCOPE_WRITE: &str = "write";

/// An unrecognised scope reads as `read`. A permission string we don't understand
/// must never be the one that widens access.
pub fn normalize_scope(s: &str) -> String {
    if s.eq_ignore_ascii_case(SCOPE_WRITE) { SCOPE_WRITE.into() } else { SCOPE_READ.into() }
}

/// Before scopes existed every connected connector could be sent through, and
/// automations were built on that. So a record written by an older build — and a
/// grant the user has not deliberately narrowed — keeps that reach; narrowing is
/// something the user opts into, never something an upgrade does behind them.
pub(super) fn default_scope() -> String {
    SCOPE_WRITE.into()
}

pub(super) fn default_enabled() -> bool {
    true
}
