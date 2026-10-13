//! The MCP server registry — the servers a user has added, persisted.
//!
//! `mcp::connectors` holds the two remote connectors we ship and can sync from.
//! This is the other half the GUI's MCP tab was written against: servers the USER
//! configures, each with its own transport config, an on/off switch and an approval
//! flag. One JSON file, because a handful of server definitions is not a database
//! and a file the user can read and hand-edit is a feature for this kind of setting.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[cfg(test)]
mod tests;

mod mcp_entry;
mod mcp_registry;

pub use mcp_entry::*;
