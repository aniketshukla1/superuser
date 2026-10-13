//! Subagent types — what a delegated subtask is allowed to be.
//!
//! A subagent exists to keep work out of the main conversation: it does one
//! self-contained job in its own context and hands back only the conclusion. The
//! pages it read and the commands it ran never enter the parent's transcript.
//!
//! A *type* decides two things: the system prompt it works under, and the tools it
//! may use. Restricting tools is the only constraint that actually holds — a type
//! listing `read_file, list_dir, grep` cannot modify anything, whatever its prompt
//! says or its task asks for.
//!
//! Types are markdown files with the same frontmatter shape as a skill, so there is
//! one format to learn. Two are built in; users add more by dropping files into
//! `subagents/` in the app data dir.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::core::agent::ToolSpec;

#[cfg(test)]
mod tests;

mod subagent_store;
mod parse_type;

pub use subagent_store::*;
pub use parse_type::*;
