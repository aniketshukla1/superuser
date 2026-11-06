//! Skills — a skill is a folder with a `SKILL.md`: YAML frontmatter (name, description,
//! optional allowed-tools) and a markdown body of instructions, the format Anthropic
//! defined. Only the catalog (each skill's name and description) goes into the agent's
//! context when a session starts; a body is fetched when the model calls `load_skill`.
//! Personas use the same frontmatter-plus-markdown shape (persona ⊇ skill).

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::core::agent::{RiskClass, ToolSpec};

#[cfg(test)]
mod tests;

mod builtin_skills;
mod skill_store;

pub use builtin_skills::*;
pub use skill_store::*;
