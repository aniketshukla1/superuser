//! Bringing an agent package across.
//!
//! A package is a folder of YAML: the bots a deployment ships, the skills they use, and the
//! deployment's own business — who may reach which bot, its branding, its model and key.
//! The first two have a home here. The rest does not, and each is named in what is left
//! behind rather than dropped without a word.
//!
//! **It is read, not run.** Nothing in the folder executes, no address in it is fetched, and
//! no environment variable it names is read: `${NAME:-default}` takes its default and
//! nothing else, because a package that could say `${AWS_SECRET_ACCESS_KEY}` and have the
//! answer written into a settings file is a package that exfiltrates.
//!
//! **Import is two steps, and what lands is what was read.** The preview carries a digest of
//! every file it read; the import re-reads the folder and refuses if the digest moved. An
//! assistant arrives switched off, like any installed one — its instructions steer a model,
//! and switching it on is the person's consent, given on its card.

use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::authoring::{self, PersonaDraft};
use super::personas::{self, PersonaRegistry};
use super::skills::SkillStore;

#[cfg(test)]
mod tests;

mod raw_agent;
mod plan;
mod apply;

pub use raw_agent::*;
pub use plan::*;
pub use apply::*;
