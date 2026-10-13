//! Which skills a session may use, and which the user muted for it.
//!
//! The composer has had a "/" popup listing this session's skills since the fork,
//! and it fetched `GET /v1/sessions/:id/skills` — a route that did not exist. So
//! the menu was always empty and there was no way for a person to invoke a skill
//! at all: they ran only when the assistant chose one off the catalogue in its
//! system prompt.
//!
//! A mute is per SESSION, not per skill. Turning something off for one
//! conversation must not turn it off everywhere — that is what the Skills settings
//! page is for — and the two get confused precisely because "disabled" is the same
//! word for both.
//!
//! Unset means available. A skill is installed because someone installed it; the
//! store records departures from that, not the whole roster, so installing a skill
//! never requires touching this file.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;

mod session_skills;

pub use session_skills::*;
