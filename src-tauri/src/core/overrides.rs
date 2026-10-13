//! User-local risk overrides — correcting what the app guessed about a tool.
//!
//! A connected app's tools arrive with no risk information at all, so
//! `commands::mcp_risk` guesses from the name: a verb like `search`, `list` or
//! `get` reads; anything else acts. That guess is right often enough to be worth
//! having and wrong in both directions:
//!
//! - **Too permissive.** `list_and_archive` contains `list`, so it classifies as a
//!   read and never sees an approval card. The user has no way to say otherwise.
//! - **Too strict.** A server whose every tool is named `run_…` gates all of them,
//!   including the ones that only read. The user's only escape is to turn on
//!   blanket auto-approval, which is a much worse outcome than the problem.
//!
//! So: a small user-owned file of glob rules. Most specific wins.
//!
//! **The rule that makes this safe to have at all: an override may never loosen a
//! built-in tool.** Relaxing `run_shell` to `read` would switch off the command
//! gate, the mode check and the approval card in one line of JSON — so a rule that
//! tries is ignored, and the built-in's own class stands. Tightening a built-in is
//! always allowed. The asymmetry is the whole design.
//!
//! **And it is never written by a persona or a skill.** A persona declares what
//! tools it wants; only the user decides how far to trust them. Nothing in this
//! file writes, which is the cheapest possible way to guarantee that.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::core::agent::RiskClass;

#[cfg(test)]
mod tests;

mod risk_overrides;
mod risk_overrides_overrides;

pub use risk_overrides::*;
