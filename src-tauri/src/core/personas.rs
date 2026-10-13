//! Personas — a persona is a specialized "assistant" defined by a markdown
//! manifest: a YAML frontmatter block declaring identity + capabilities, followed
//! by a markdown body that IS the system prompt. The same frontmatter↔markdown
//! shape is reused by skills (persona ⊇ skill).

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;

mod grantable_tools;
mod manifest;
mod registry;

pub use grantable_tools::*;
pub use manifest::*;
pub use registry::*;
