use super::*;

pub(super) fn now() -> f64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Verdict {
    Up,
    Down,
}

/// What was wrong with a turn, from a fixed list.
///
/// Fixed, and short, on purpose. Free text alone cannot be folded — a hundred
/// sentences are a hundred sentences — and a long list is one nobody reads to the
/// end of, so the first option becomes the answer to everything.
///
/// Each arm is chosen to name a *part of this system*, so a count against it points
/// at a file rather than at a mood.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    /// Stated something that was not in any source it had. → `gates.rs`, the
    /// grounding and citation checks, and the honesty tasks on the scoreboard.
    MadeItUp,
    /// Used the wrong file, page or memory — the answer's *shape* was right and it
    /// was built on the wrong thing. → retrieval: `embeddings.rs`, `coder.rs` paths.
    WrongSource,
    /// Ignored something the user had already told it. → `supercontext.rs` standing
    /// rules, `profile.rs`, the core memory.
    IgnoredInstruction,
    /// Right facts, wrong form: too long, wrong format, did not do what was asked
    /// with them. → the persona prompt and `gates::prompt_block`.
    WrongShape,
    /// Correct, and took far too long. → `loopshape.rs`, speculation, the tool set.
    TooSlow,
    /// None of the above. Always present, because a fixed list that cannot be
    /// escaped collects lies.
    Other,
}

impl Reason {
    pub const ALL: &'static [Reason] =
        &[Reason::MadeItUp, Reason::WrongSource, Reason::IgnoredInstruction, Reason::WrongShape, Reason::TooSlow, Reason::Other];

    /// The words the user sees. Written as complaints in their voice, not as system
    /// components — nobody has ever thought "this is a grounding failure".
    pub fn label(self) -> &'static str {
        match self {
            Reason::MadeItUp => "It made something up",
            Reason::WrongSource => "It used the wrong file or memory",
            Reason::IgnoredInstruction => "It ignored what I'd told it",
            Reason::WrongShape => "Right answer, wrong form",
            Reason::TooSlow => "It took far too long",
            Reason::Other => "Something else",
        }
    }

    /// Where to look first. Shown in the report, never to the user.
    pub fn points_at(self) -> &'static str {
        match self {
            Reason::MadeItUp => "gates.rs — grounding, citation, honesty tasks",
            Reason::WrongSource => "embeddings.rs recall, coder.rs paths",
            Reason::IgnoredInstruction => "supercontext.rs standing rules, profile.rs, core memory",
            Reason::WrongShape => "persona prompt, gates::prompt_block",
            Reason::TooSlow => "loopshape.rs, speculation.rs, the tool set",
            Reason::Other => "read the note",
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Reason::MadeItUp => "made_it_up",
            Reason::WrongSource => "wrong_source",
            Reason::IgnoredInstruction => "ignored_instruction",
            Reason::WrongShape => "wrong_shape",
            Reason::TooSlow => "too_slow",
            Reason::Other => "other",
        }
    }

    pub fn parse(s: &str) -> Option<Reason> {
        Reason::ALL.iter().copied().find(|r| r.as_str() == s)
    }
}

/// What the turn did, captured by us rather than described by the user.
///
/// The user says a turn was wrong; this says what it *did*. Without it a downvote is
/// a mood, and with it "wrong source, and `recall_memory` returned nothing" is a
/// bug report someone can start on.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Evidence {
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub persona: String,
    /// Tools this turn called, in order, deduplicated.
    #[serde(default)]
    pub tools: Vec<String>,
    #[serde(default)]
    pub steps: usize,
    #[serde(default)]
    pub seconds: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub at: f64,
    pub session: String,
    /// Index of the assistant message in that session's transcript. Messages carry
    /// no id, and the transcript is never rewritten — `/compact` changes only what
    /// the next run is told — so the index is stable. `message_at` is kept beside it
    /// so a record whose index no longer lines up can be spotted rather than
    /// silently misattributed.
    pub message: usize,
    #[serde(default)]
    pub message_at: f64,
    pub verdict: Verdict,
    /// Absent on an upvote: "this was good" needs no taxonomy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<Reason>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub note: String,
    #[serde(default)]
    pub evidence: Evidence,
}

pub(super) fn path(data_dir: &Path) -> PathBuf {
    data_dir.join("feedback.jsonl")
}

/// Append one verdict. Never panics and never fails the caller: a feedback write
/// must not be able to break the conversation it is about.
pub fn record(data_dir: &Path, entry: &Entry) {
    let Ok(line) = serde_json::to_string(entry) else { return };
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(path(data_dir)) {
        let _ = writeln!(f, "{line}");
    }
}

/// Everything recorded, oldest first.
pub fn all(data_dir: &Path) -> Vec<Entry> {
    std::fs::read_to_string(path(data_dir))
        .unwrap_or_default()
        .lines()
        .filter_map(|l| serde_json::from_str::<Entry>(l).ok())
        .collect()
}

/// The current verdict on one message, if the user has given one.
///
/// Last write wins: a person who changes their mind has changed their mind, and the
/// earlier record stays on disk because the file is append-only.
pub fn verdict_for(entries: &[Entry], session: &str, message: usize) -> Option<Verdict> {
    entries.iter().rev().find(|e| e.session == session && e.message == message).map(|e| e.verdict)
}

/// What the feedback adds up to.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Report {
    pub up: usize,
    pub down: usize,
    /// Downvotes by reason, most-complained-about first.
    pub by_reason: Vec<(Reason, usize)>,
    /// Downvotes by model, so "the harness is bad" and "this model is bad" stop
    /// being the same sentence.
    pub by_model: Vec<(String, usize)>,
    /// Tools present on downvoted turns, most frequent first.
    ///
    /// Presence, not blame: `read_file` appears on most turns, so it will appear on
    /// most bad ones. It earns attention when its share here is higher than its
    /// share everywhere, which is a comparison against `toolstats` and not something
    /// this number claims on its own.
    pub tools_on_downvotes: Vec<(String, usize)>,
}
