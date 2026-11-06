//! The gateway — the one door every tool call passes through.
//!
//! Its job is to guarantee that nothing acts without a row: the decision is written
//! down before the action runs, on every path, so an action with no row never happened.
//!
//! Before this, the decision about an action was spread through the tools: the
//! command gate inside `run_shell`, the send check inside `send_message`, the MCP
//! check inside the connector branch, and an audit line written after some actions
//! and not others. Each was right on its own. Nothing guaranteed that a new tool
//! would get any of them, and there was no single place a person's rule could be
//! applied to *every* action.
//!
//! Now `commands::run_tool_with` calls [`before`] for every call that survived the
//! hallucination gates, runs the tool only if it says so, and calls [`after`] with the
//! result. In order:
//!
//! 1. **What is this action?** A [`Subject`], derived here from the tool and its
//!    arguments — never from a label the caller supplied — with its `intent`: what
//!    the action does, not which tool does it. An operator thinks in effects ("never
//!    send"), and mechanism is a poor proxy for effect.
//! 2. **What do the user's boundaries say?** `core::boundaries`, deny before allow,
//!    fail closed.
//! 3. **Write the decision down** — before anything runs.
//! 4. **Act**, which is where the approval engine still asks, parks or runs: the
//!    boundaries only ever take permission away.
//! 5. **Write the outcome down** when it was not simply done: failed, stopped by the
//!    user, parked for approval, or declined by the mode or by the user.
//!
//! A refusal is a sentence the assistant can say, never an exception: an assistant
//! mid-run with a person waiting is owed something it can say.

use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;

use serde_json::Value;

use crate::core::agent::RiskClass;
use crate::core::audit::{self, Decision, Entry, Initiator, Subject};
use crate::core::boundaries;

#[cfg(test)]
mod tests;

mod intent_of;
mod subject_for;
mod outcome_of;

pub use intent_of::*;
pub use subject_for::*;
pub use outcome_of::*;
