use super::*;

/// What a pattern records. The distinction is not cosmetic: a failure is a thing to
/// stop happening, a strategy is a thing to keep doing, and a reviewer told to fix
/// the first will happily delete the second.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// A way the work went wrong, with what would have avoided it.
    Failure,
    /// A way the work went right that is worth repeating.
    Strategy,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Failure => "failure",
            Kind::Strategy => "strategy",
        }
    }

    /// Anything unrecognised reads as a failure. A reviewer that mistypes the kind
    /// should still get its observation recorded, and a mislabelled failure costs
    /// less than a dropped one.
    pub fn parse(s: &str) -> Kind {
        match s.trim().to_ascii_lowercase().as_str() {
            "strategy" | "success" | "win" => Kind::Strategy,
            _ => Kind::Failure,
        }
    }
}

/// One page under `patterns/`: a single failure mode or working strategy.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Pattern {
    /// Slug, and the filename stem. Stable across updates — that is what makes a
    /// second sighting reinforce a page instead of creating a near-duplicate.
    pub id: String,
    pub title: String,
    pub kind: Kind,
    /// How many passes have now observed this. A pattern seen repeatedly is the
    /// strongest available signal that a skill needs changing.
    pub seen: u32,
    pub first_seen: String,
    pub updated: String,
    pub body: String,
}

/// What happened to a change a review proposed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Verdict {
    /// Written to disk and left there.
    Accepted,
    /// The user undid it. The row survives so it is never proposed again.
    Undone,
}

impl Verdict {
    pub fn as_str(self) -> &'static str {
        match self {
            Verdict::Accepted => "accepted",
            Verdict::Undone => "undone",
        }
    }
}

/// A row of the proposal ledger — the record the paper calls `skill-impact.md`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Impact {
    pub at: String,
    /// The file the pass changed, as a short name (`release`, `MEMORY.md`).
    pub target: String,
    /// Pattern ids that motivated it.
    pub patterns: Vec<String>,
    pub verdict: Verdict,
    /// The snapshot the change belongs to — how an undo finds its rows again.
    pub snapshot: String,
    pub note: String,
}

/// Markdown tables are the wiki's storage as well as its presentation, so a cell
/// containing a pipe or a newline would silently split a row in two.
pub(super) fn cell(s: &str) -> String {
    s.replace('|', "\\|").replace(['\n', '\r'], " ").trim().to_string()
}

pub(super) fn today() -> String {
    crate::core::journal::today()
}

pub(super) fn stamp() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M").to_string()
}

/// Frontmatter fields kept across an update. The reviewer supplies title, kind and
/// body; everything here is bookkeeping it must not be able to reset.
#[derive(Debug, Default, Deserialize)]
pub(super) struct Front {
    pub(super) title: Option<String>,
    pub(super) kind: Option<String>,
    pub(super) seen: Option<u32>,
    pub(super) first_seen: Option<String>,
    pub(super) updated: Option<String>,
}

/// Whether a write created a page or reinforced one that already existed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wrote {
    Created,
    Reinforced,
}

pub struct Wiki {
    pub root: PathBuf,
}
