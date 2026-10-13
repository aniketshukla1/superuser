//! Installing an assistant that someone else wrote.
//!
//! A persona manifest is two things at once: a system prompt the model will obey,
//! and a list of capabilities it may reach for. Installing one is closer to hiring
//! than to downloading — whoever wrote it gets to steer a process that can read
//! your files and, if the manifest asks for `shell`, run commands.
//!
//! So the trust model is the one Settings ▸ Personas already promises in words,
//! and this module is what makes those words true:
//!
//! * **Files are copied into a managed area.** What runs is a snapshot under
//!   `data_dir/personas/`, not a path that keeps pointing at a repo somebody else
//!   can edit tomorrow.
//! * **No code runs.** Nothing here executes a fetched file, and the git path is
//!   plain HTTPS against the GitHub API rather than `git clone` — a clone is a
//!   program with a URL parser, submodule handling and transport plugins, and none
//!   of that is needed to read some Markdown.
//! * **Installs land disabled.** `PersonaRegistry::is_enabled` defaults a
//!   non-built-in to off, so an installed assistant cannot be started until the
//!   user has seen the consent card and turned it on. That default is the whole
//!   safety property of this feature; do not "helpfully" enable here.
//!
//! Deliberately narrow in this first cut: **manifests only**. A persona bundle can
//! carry reference files (`<id>-files/`), and copying those means copying arbitrary
//! content the assistant will later read as fact. That is a separate consent
//! question, so a bundle's extra files are reported and skipped rather than
//! silently installed.

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::core::personas::{parse_manifest, PersonaManifest, PersonaRegistry};

#[cfg(test)]
mod tests;

mod install;
mod from_git;

pub use install::*;
use from_git::*;
