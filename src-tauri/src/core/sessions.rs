//! Session store — a conversation with an assistant, persisted so it survives
//! restarts. The GUI's sidebar lists sessions, its transcript replays a session's
//! message history, and its socket streams the live turn.
//!
//! One JSON file per session under `data_dir/sessions/<id>/session.json`, beside
//! that session's working folder (where its artifacts land), so deleting a
//! session takes its deliverables with it.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[cfg(test)]
mod folder_tests;

#[cfg(test)]
mod tests;

mod token_usage;
mod session;
mod session_sessions;
mod split_tasks;
mod session_store;

pub use token_usage::*;
pub use session::*;
pub use split_tasks::*;
