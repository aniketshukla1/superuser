//! Semantic recall: embeds memories + queries with a local Ollama embedding
//! model and ranks by cosine similarity. The vector index is a JSON file in the
//! app data dir, keyed by filename with a content hash so edits re-embed.
//! Falls back to keyword recall whenever Ollama is unreachable.

use super::memory::{Citation, MemoryEngine};
use super::settings::Settings;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[cfg(test)]
mod tests;

mod embed;
mod semantic;
mod journal_recall;

pub use embed::*;
pub use semantic::*;
pub use journal_recall::*;
