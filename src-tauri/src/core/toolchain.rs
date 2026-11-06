//! The scanner toolchain — what is installed, what is missing, and what a
//! missing one costs the answer.
//!
//! This module exists for one failure mode, and it is worth stating plainly
//! because it is the worst outcome a review can produce. A security pass is
//! asked to sweep git history for leaked credentials. The tool that does that
//! sweep is not on the machine. The check does not fail — it *vanishes*. The
//! report comes back clean, and "we could not look" has been silently rendered
//! as "there is nothing there." The user reads a green report and believes it.
//!
//! So every check ends one of two ways: it **ran**, or the report **says it didn't**. There
//! is no third option, and in particular there is no option where the model simply
//! forgets. That is why [`check_tool`](crate::core::agent_exec) writes the
//! ledger itself, at the moment it discovers a tool is absent, rather than
//! trusting the model to remember and disclose it later. The model can amend an
//! entry (it fell back to a manual sweep, so the check is *degraded* rather than
//! *skipped*) but it cannot make one disappear.
//!
//! The catalogue is closed on purpose. These names are interpolated into a shell
//! probe, so an open set would be an injection hole; and a fixed list is what
//! lets [`Tool::fallback`] say something true about what is lost when one is
//! missing.

use std::process::{Command, Stdio};

use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;

mod catalog;

pub use catalog::*;
