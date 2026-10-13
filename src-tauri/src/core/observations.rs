//! Addressable recall for truncated tool output (HARNESS-RESEARCH-2026 §2).
//!
//! `context::relieve_pressure` rewrites an over-long earlier tool result to head +
//! tail so the window keeps its shape. The notice it left behind used to say
//! *"re-read the source if you need the middle"*, which presumes the source is
//! still there to re-read. For `read_file` it usually is. For `run_shell` it
//! usually is not: the build that took ninety seconds, the `git log` on a tree
//! that has since changed, an MCP call against a rate limit, anything with a side
//! effect. We were telling the model to go and fetch something that no longer
//! existed, and paying for it twice — once as a wasted step, then again as a wrong
//! answer.
//!
//! So the full text is written here *before* it is cut, and the notice names a
//! handle instead. Recovery becomes a decision the model makes rather than a
//! re-derivation it may get wrong.
//!
//! Three properties keep this honest, and each is a rule rather than a nicety:
//!
//! * **Content-addressed.** The id is a hash of the bytes that were truncated, so
//!   a recall returns what the model actually saw. If the same output appears
//!   twice it is stored once; if a file changed underneath, the old handle still
//!   yields the old text rather than quietly resolving to the new one. That is the
//!   whole difference between this and re-reading.
//! * **Per session, and it dies with the session.** [`ObservationStore::clear`] is
//!   what `SessionStore::delete` calls; nothing here outlives the conversation it
//!   belongs to.
//! * **Read-only.** The `recall_output` tool is `RiskClass::Read`, so nothing in the
//!   approval or taint model moves to accommodate it.
//!
//! Bounded on purpose: [`MAX_BYTES`] caps a session's store, and a put that would
//! exceed it is refused rather than evicting something a live handle points at. A
//! refused put means the notice falls back to the old wording for that one result,
//! which is the pre-existing behaviour and not a regression.

use std::fs;
use std::path::{Path, PathBuf};

#[cfg(test)]
mod tests;

mod observation_store;

pub use observation_store::*;
