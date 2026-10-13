use super::*;

pub(super) fn now() -> f64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0)
}

pub(super) const DAY: f64 = 86_400.0;

/// What a preference is *about*.
///
/// Each class carries its own half-life and its own budget, so one talkative class
/// can't crowd the others out of a prompt. People, topics and threads deliberately
/// aren't here: those are memory, and they belong to the journal and the vault.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Class {
    /// Who they are: name, role, where they work, what they're responsible for.
    Identity,
    /// How to talk to them: length, tone, formatting, how much to explain.
    Style,
    /// What they work with: languages, editors, stacks, conventions.
    Tooling,
    /// What not to do. Kept separate from Style because a veto is a rule, and a
    /// rule that decays at the speed of a tone preference is a rule that will be
    /// broken.
    Veto,
    /// What they're trying to get done over a longer arc than one conversation.
    Goal,
    /// Where to reach them, and how they want to be interrupted.
    Channel,
}

pub const CLASSES: [Class; 6] =
    [Class::Identity, Class::Style, Class::Veto, Class::Tooling, Class::Goal, Class::Channel];

impl Class {
    pub fn slug(self) -> &'static str {
        match self {
            Class::Identity => "identity",
            Class::Style => "style",
            Class::Tooling => "tooling",
            Class::Veto => "veto",
            Class::Goal => "goal",
            Class::Channel => "channel",
        }
    }

    pub fn parse(s: &str) -> Option<Class> {
        CLASSES.into_iter().find(|c| c.slug() == s.trim().to_lowercase())
    }

    /// The heading this class renders under, in PROFILE.md and in the prompt.
    pub fn heading(self) -> &'static str {
        match self {
            Class::Identity => "Identity",
            Class::Style => "Style",
            Class::Tooling => "Tooling",
            Class::Veto => "Do not",
            Class::Goal => "Goals",
            Class::Channel => "Channels",
        }
    }

    /// How long a single piece of evidence keeps half its weight.
    ///
    /// The spread is the point: who someone is barely changes, how they want a
    /// paragraph written changes constantly, and a preference nobody has restated
    /// in two months is a guess. A style note therefore has to be re-earned every
    /// few weeks or it fades — which is the behaviour you want from something that
    /// silently steers every answer. Pinning is the escape hatch for the ones that
    /// really are permanent.
    pub fn half_life_days(self) -> f64 {
        match self {
            Class::Identity => 90.0,
            Class::Veto => 60.0,
            Class::Tooling | Class::Goal => 30.0,
            Class::Style => 14.0,
            Class::Channel => 7.0,
        }
    }

    /// How many facets of this class may reach the prompt.
    pub fn budget(self) -> usize {
        match self {
            Class::Identity | Class::Tooling => 6,
            Class::Style | Class::Veto => 5,
            Class::Goal => 4,
            Class::Channel => 3,
        }
    }
}

/// How a preference was learned. The ordering is the whole point: what someone
/// said outright outranks what we inferred from watching them, so a single "call
/// me Nik" beats any amount of guessing.
///
/// Three families, not the four the design sketched. The fourth was `Recurrence`
/// — a preference earning weight by happening repeatedly — but the scorer already
/// sums every observation of the same facet, so recurrence *is* how the score
/// grows. Adding it as its own family would have counted the same signal twice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Cue {
    /// They said it: "call me Nik", "stop apologizing", "never email my manager".
    Explicit,
    /// It follows from data rather than from prose — the profile form, a connected
    /// account's own settings.
    Structural,
    /// Inferred from how the conversation went. The weakest, and it has to happen
    /// more than once before it reaches a prompt.
    Behavioral,
}

impl Cue {
    pub fn weight(self) -> f64 {
        match self {
            Cue::Explicit => 2.0,
            Cue::Structural => 1.0,
            Cue::Behavioral => 0.6,
        }
    }

    pub fn parse(s: &str) -> Option<Cue> {
        match s.trim().to_lowercase().as_str() {
            "explicit" | "said" | "stated" => Some(Cue::Explicit),
            "structural" | "data" => Some(Cue::Structural),
            "behavioral" | "behavioural" | "inferred" => Some(Cue::Behavioral),
            _ => None,
        }
    }
}

/// One piece of evidence, kept forever (up to the store's cap) and never edited.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    pub class: Class,
    /// What this is a preference *about* — a slug, so two phrasings of the same
    /// preference land on the same facet and compete rather than both rendering.
    pub key: String,
    /// The preference itself, in a form that can be read aloud in a prompt.
    pub value: String,
    pub cue: Cue,
    pub at: f64,
    /// The user's own words, kept so the Profile surface can answer "why does it
    /// think that?" — the same evidence-before-interpretation rule as Step 2.
    #[serde(default)]
    pub evidence: String,
    /// Where it came from: a session id, or `form`.
    #[serde(default)]
    pub source: String,
}

/// Where a facet stands right now. Derived, never stored — which is what lets a
/// dropped belief re-earn its place simply by being observed again.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    /// Rendered into PROFILE.md and folded into the prompt.
    Active,
    /// Tracked and visible in the app, but not acted on: either not enough
    /// evidence yet, or beaten by a competing value for the same key.
    Provisional,
}

/// A belief about the user, assembled from every observation that supports it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Facet {
    pub class: Class,
    pub key: String,
    pub value: String,
    pub state: State,
    /// Decayed sum of the supporting evidence.
    pub score: f64,
    pub cue: Cue,
    pub observations: usize,
    pub last_seen: f64,
    pub evidence: String,
    pub pinned: bool,
    /// Straight from the profile form. Such a facet is Active without having to
    /// clear the evidence threshold — the user typed it, so it applies — but it
    /// still competes for its key like anything else, which is what lets someone
    /// override a stale form field by saying so rather than by going to edit it.
    pub from_form: bool,
}

/// The score a facet needs before it is allowed to steer an answer.
///
/// Set so that one explicit statement is enough on its own — being told something
/// once should work — while a single behavioural guess (0.6) is not. Two
/// independent guesses reach it; one never does.
pub(super) const ACTIVE_AT: f64 = 1.0;

/// Kept observations. Generous, because they're small and the decay curve is what
/// actually retires a belief — the cap only stops an always-on app growing a file
/// forever.
pub(super) const OBSERVATION_LIMIT: usize = 2_000;
