//! Talking to somebody else's agent — the AG-UI client.
//!
//! The server half lets a Superuser assistant be the agent behind another app's chat
//! window. This is the other direction: an assistant here whose turns are actually run by
//! a remote AG-UI agent — LangGraph, Mastra, Pydantic AI, a hand-written service — with
//! this app supplying the conversation and showing the answer.
//!
//! **What this app can promise about a remote agent is small, and saying so is the point.**
//! Its tools run on its own machine, under its own rules; our gateway, boundaries and
//! approvals do not reach inside it. So the two things we CAN hold are held here: where it
//! is allowed to be (the endpoint floor, checked again on every redirect, with credentials
//! dropped when a hop changes scope) and what its answer counts as (content from off this
//! machine — the run that carries it is tainted, exactly like a fetched page).

use std::time::Duration;

use serde_json::{json, Value};

#[cfg(test)]
mod tests;

mod reader;
mod run_under;

pub use reader::*;
pub use run_under::*;
