//! Skills — Anthropic SKILL.md format with progressive disclosure. A skill is a
//! folder containing `SKILL.md` (YAML frontmatter: name, description, optional
//! allowed-tools) + a markdown body of instructions. At session start only the
//! catalog (name + description) is injected into the agent's context; the full body
//! is loaded on demand via the `load_skill` tool. Same frontmatter↔markdown shape
//! as a persona (persona ⊇ skill).

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
