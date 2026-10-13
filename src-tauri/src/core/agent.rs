//! Agent core — the runtime side of a persona.
//!
//! A persona declares a set of capability ids (its `tools:` list). The **catalog**
//! is the platform-owned, closed mapping from a capability id to the concrete tools
//! it contributes, plus the session context each needs and the risk classes it can
//! produce. `expand(ids, ctx)` turns a persona's tool list into the concrete tool
//! specs offered to the model for a given session — skipping capabilities whose
//! context prerequisites aren't met (no shell without an executor, no files without
//! a workspace). MCP tools are NOT in the catalog; they attach separately.
//!
//! This module defines the *contract* (tool schemas, risk, context gating). The
//! model-calling loop that dispatches these tool calls to handlers and produces
//! deliverables is built on top of it.
#![allow(dead_code)] // foundation APIs consumed by the agent loop (next increment)

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::core::personas::PersonaManifest;

#[cfg(test)]
mod tests;

mod resident;
mod capability_builders;
mod agent;
mod todo_list;

pub use resident::*;
pub use capability_builders::*;
pub use agent::*;
pub use todo_list::*;
