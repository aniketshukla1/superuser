//! Self-evolution: a quiet review of a conversation after it goes idle.
//!
//! Everything else in the memory system records what happened. This decides
//! whether anything about *how the assistant works* should change because of it —
//! a skill that showed a flaw, a durable fact the main chat missed — and then makes
//! that change.
//!
//! Four properties do the heavy lifting, and each is enforced in code rather than
//! asked for in the prompt:
//!
//! * **Doing nothing is the normal outcome.** Most conversations warrant no change.
//!   The prompt says so; `Outcome::Silent` is the common return.
//! * **Nothing outside the workspace can be touched.** Edits are applied through a
//!   path guard against an explicit allowlist (user skills, the journal). Built-in
//!   skills are compiled into the binary with `include_str!`, so they are
//!   unmodifiable by construction rather than by policy.
//! * **No work means no message.** Whether the user is told is decided by comparing
//!   file snapshots, not by whether the model claimed it did something. A model that
//!   narrates an edit it never made stays silent.
//! * **Every pass is reversible.** The files it may touch are snapshotted first, so
//!   "undo that" restores exactly the prior state, including files that did not
//!   exist before.
//!
//! The prompt's do-not-capture list matters most: it encodes hard-won failure modes.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::core::journal::Journal;
use crate::core::settings::Settings;
use crate::core::wiki::{self, Wiki};

#[cfg(test)]
mod tests;

mod restore;
mod when_run;
mod review_pass;

pub use restore::*;
pub use when_run::*;
pub use review_pass::*;
