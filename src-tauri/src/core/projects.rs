//! Projects: named containers that group related chats, optionally backed by a
//! folder on the local disk. Stored as plain JSON next to the conversation
//! store. The folder path is chosen by the user through the OS dialog (which is
//! where the "grant access to this folder" permission happens), so all we keep
//! here is the path string.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[cfg(test)]
mod tests;

mod create;

pub use create::*;
