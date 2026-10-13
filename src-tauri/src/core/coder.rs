//! Coder workspace tools: a real file tree, file reads, git status, and a
//! workspace-scoped grep. Everything is path-checked to stay inside the chosen
//! workspace root. (Phase 1 is read-only; editing comes in phase 2.)

use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(test)]
mod tests;

mod missing_path;
mod str_replace;
mod run_capture;
mod source_control;

pub use missing_path::*;
pub use str_replace::*;
pub use run_capture::*;
pub use source_control::*;
