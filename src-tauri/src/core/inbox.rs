//! Inbox + permission engine. The model:
//! writes, sends and shell are approval-gated, and an **unattended** run (a
//! scheduled automation) never acts on its own — it *parks its ask* in the inbox
//! and stops, so the user resolves it later instead of the agent deciding alone.
//!
//! A standing rule ("always allow send_message #ops") can pre-approve one exact
//! tool+target pair; rules live on the automation so revoking is per-task.
//!
//! A standing rule pre-approves an *action*, though, and an action is only as safe
//! as whatever asked for it. Once a run has read content from outside this machine
//! — a synced email, a web page, an MCP result — the run is **tainted**, and the
//! pre-approval no longer applies to anything that can reach back out. See
//! [`decide`].

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::core::agent::RiskClass;

#[cfg(test)]
mod tests;

mod decide;
mod inbox;

pub use decide::*;
