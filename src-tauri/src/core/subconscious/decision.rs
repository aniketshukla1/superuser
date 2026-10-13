use super::*;

pub(super) fn now() -> f64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0)
}

pub(super) fn new_id(prefix: &str) -> String {
    format!("{prefix}-{}", (now() * 1000.0) as u64)
}

/// What the local model decided about one watch on one tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Decision {
    /// Nothing here is worth spending anything on.
    Skip,
    /// Worth answering now, on the local model.
    Act,
    /// Needs more than this loop should decide alone.
    Escalate,
}

impl Decision {
    /// Parse a model's answer. Unrecognised → `Skip`, deliberately: a loop that
    /// runs unattended every few minutes must treat an answer it can't read as a
    /// reason to do nothing, not as licence to act.
    pub fn parse(raw: &str) -> Self {
        Self::read(raw).0
    }

    /// The decision and the reason given for it.
    ///
    /// Small local models answer this prompt in three shapes: the word alone, the
    /// word then a reason on the next line, and the word and reason on one line
    /// ("skip — nothing relevant"). All three have to work, because the reason is
    /// what makes the activity log worth reading; a log of bare verdicts explains
    /// nothing about why the loop ignored the email you cared about.
    pub fn read(raw: &str) -> (Self, String) {
        let trimmed = raw.trim();
        let first = trimmed.lines().next().unwrap_or("").trim();
        let cleaned = first.trim_start_matches(['-', '*', '#', ' ', '"', '\'']);
        let word = cleaned.to_lowercase();

        let (decision, len) = if word.starts_with("escalate") {
            (Decision::Escalate, "escalate".len())
        } else if word.starts_with("act") {
            (Decision::Act, "act".len())
        } else if word.starts_with("skip") {
            (Decision::Skip, "skip".len())
        } else {
            // Unrecognised: still Skip, and the whole line becomes the reason —
            // "the model said something we couldn't read" is worth seeing in the log.
            (Decision::Skip, 0)
        };

        // Whatever followed the verdict on its own line, else the next line that
        // has anything on it.
        let tail = cleaned[len.min(cleaned.len())..].trim_start_matches([':', '—', '-', '.', ' ']).trim();
        let reason = if !tail.is_empty() {
            tail.to_string()
        } else {
            trimmed.lines().skip(1).map(str::trim).find(|l| !l.is_empty()).unwrap_or("").to_string()
        };
        (decision, reason.chars().take(240).collect())
    }
}

/// A standing question about one or more sources.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Watch {
    pub id: String,
    /// What to watch for, in the user's words. This is the prompt.
    pub question: String,
    /// Connector ids whose changes feed it. Empty means the whole vault.
    #[serde(default)]
    pub sources: Vec<String>,
    #[serde(default = "tru")]
    pub enabled: bool,
    /// True when answering it could mean acting outside this machine (sending,
    /// posting, filing). Read-only watches can be answered on the spot; a
    /// write-intent one always ends at the Inbox when nobody is watching.
    #[serde(default)]
    pub writes: bool,
    #[serde(default)]
    pub created_at: f64,
    #[serde(default)]
    pub last_tick: Option<f64>,
}

pub(super) fn tru() -> bool {
    true
}

impl Watch {
    pub fn new(question: String, sources: Vec<String>, writes: bool) -> Self {
        Self {
            id: new_id("watch"),
            question,
            sources,
            enabled: true,
            writes,
            created_at: now(),
            last_tick: None,
        }
    }
}

/// One line in the ledger of what the loop did. The point of writing these down is
/// that a background process nobody can inspect is a background process nobody
/// should trust — including "I looked and there was nothing", which is most of them.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Activity {
    pub id: String,
    pub watch_id: String,
    pub question: String,
    pub at: f64,
    pub decision: Decision,
    /// The model's one-line justification, or the reason no model ran.
    pub reason: String,
    /// What the delta looked like, e.g. "gmail: 3 new".
    #[serde(default)]
    pub evidence: String,
    /// The answer, when the decision was `act`.
    #[serde(default)]
    pub note: Option<String>,
    /// The inbox item raised, when the decision was `escalate`.
    #[serde(default)]
    pub ask_id: Option<String>,
}

/// Watches and their activity, persisted beside the other stores.
pub struct Subconscious {
    pub(super) data_dir: PathBuf,
    pub watches: Vec<Watch>,
}

/// How many activity lines to keep. Enough to answer "what has it been doing",
/// bounded so an always-on loop can't grow a file forever.
pub(super) const ACTIVITY_LIMIT: usize = 500;
