//! Personas — each assistant is described by one markdown file: YAML frontmatter
//! for who it is and what it may use, then a markdown body that becomes its system
//! prompt word for word. The same frontmatter↔markdown
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
