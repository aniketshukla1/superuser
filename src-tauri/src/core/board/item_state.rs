use super::*;

pub(super) fn now() -> f64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemState {
    Open,
    InProgress,
    Blocked,
    Review,
    Done,
    Canceled,
}

impl ItemState {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().replace('-', "_").as_str() {
            "open" => Some(Self::Open),
            "in_progress" | "inprogress" | "doing" => Some(Self::InProgress),
            "blocked" => Some(Self::Blocked),
            "review" => Some(Self::Review),
            "done" => Some(Self::Done),
            "canceled" | "cancelled" => Some(Self::Canceled),
            _ => None,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::InProgress => "in_progress",
            Self::Blocked => "blocked",
            Self::Review => "review",
            Self::Done => "done",
            Self::Canceled => "canceled",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EventKind {
    Created { title: String, detail: String, acceptance: String },
    Assigned { to: String },
    Moved { to: ItemState },
    Noted { text: String },
    Reviewed { pass: bool, evidence: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub at: f64,
    pub item: String,
    /// The run that did this — a session id, assigned by the platform.
    pub actor: String,
    #[serde(flatten)]
    pub kind: EventKind,
}

#[derive(Debug, Clone, Serialize)]
pub struct Review {
    pub by: String,
    pub pass: bool,
    pub evidence: String,
}

/// One work item, folded from the log.
#[derive(Debug, Clone, Serialize)]
pub struct Item {
    pub id: String,
    pub title: String,
    pub detail: String,
    /// What "done" means for this item, written when it was created. An item
    /// without it can be reviewed only on someone's opinion.
    pub acceptance: String,
    pub state: ItemState,
    pub assignee: Option<String>,
    /// Who moved it to review — the one actor barred from passing it.
    pub builder: Option<String>,
    pub notes: Vec<String>,
    pub reviews: Vec<Review>,
}

impl Item {
    /// Has someone other than the builder passed it?
    pub fn verified(&self) -> bool {
        self.reviews.iter().any(|r| r.pass && Some(&r.by) != self.builder.as_ref())
    }
}

pub(super) fn slug(workspace: &Path) -> String {
    let s: String = workspace
        .to_string_lossy()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect();
    let trimmed = s.trim_matches('-').to_string();
    // Long paths make long filenames; the tail is the distinguishing part.
    let out = if trimmed.len() > 80 { trimmed[trimmed.len() - 80..].to_string() } else { trimmed };
    if out.is_empty() { "board".into() } else { out }
}

/// One project's board.
pub struct Board {
    pub(super) path: PathBuf,
}
