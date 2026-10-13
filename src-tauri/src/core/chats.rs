//! Conversation store + chat→memory indexing. Conversations are saved as
//! structured JSON (reopenable history); on indexing, each is summarized by the
//! local model and written into the vault as a searchable "Chat" memory.

use super::memory::{MemoryEngine, MemoryInput, Origin};
use super::model_router::{self, ChatMessage};
use super::settings::Settings;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[cfg(test)]
mod tests;

mod index;

pub use index::*;
