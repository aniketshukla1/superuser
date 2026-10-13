use super::*;

/// How long an unanswered ask stays on screen. Long enough that someone who stepped away
/// can still act on it; short enough that it does not follow the assistant into
/// tomorrow's conversations. Expired on read — there is nothing to wake.
pub const ASK_TTL_SECS: f64 = 10.0 * 60.0;

pub const HUMAN_HAS_CONTROL: &str =
    "The user has control of your browser right now. Wait for them to hand it back — don't act on the page, and don't retry this in a loop.";

pub const TAKE_CONTROL_FIRST: &str = "Take control before driving the browser yourself.";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Holder {
    Assistant,
    User,
}

/// One value the assistant needs and must never see.
#[derive(Debug, Clone, PartialEq)]
pub struct SecretAsk {
    pub label: String,
    pub r#ref: String,
    /// The element the ref named when it was asked for. Kept rather than re-resolved, so
    /// a value can only ever go to the field the assistant pointed at.
    pub backend_node_id: i64,
    pub at: f64,
}

/// The state as the GUI reads it. Never carries a secret, and never the time a secret
/// was asked for — one fewer thing on a screen that is asking someone for a password.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ControlView {
    pub holder: Holder,
    pub since: f64,
    /// Why the assistant asked, so the user knows what they are being handed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// The assistant has asked for help and nobody has taken the wheel yet.
    pub requested: bool,
    /// The label of a value the assistant is waiting for, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret_wanted: Option<String>,
}

impl ControlView {
    /// Something here wants the user: an ask for help, or a value.
    pub fn needs_you(&self) -> bool {
        self.requested || self.secret_wanted.is_some()
    }
}

#[derive(Debug, Clone)]
pub struct Control {
    pub(super) holder: Holder,
    pub(super) since: f64,
    pub(super) reason: Option<String>,
    pub(super) requested_at: Option<f64>,
    pub(super) secret: Option<SecretAsk>,
    /// Bumped each time the user hands control back, so a waiting tool can tell "they
    /// took it and returned it" from "nobody came".
    pub(super) releases: u64,
    /// Bumped each time a secret lands in its field.
    pub(super) supplied: u64,
    /// Bumped each time the user says "not now" to a request for help — an answer, and not
    /// the same one as nobody coming.
    pub(super) declines: u64,
}
