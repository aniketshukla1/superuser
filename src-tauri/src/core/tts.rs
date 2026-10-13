//! Speech out — the voice half of a spoken conversation.
//!
//! Two ideas carry most of the weight here, and both are about *time* rather than
//! audio quality.
//!
//! **Chunking.** A reply is spoken as it is written, not after. The model streams
//! tokens; `next_chunk` hands back a whole sentence the moment one exists, so the
//! first words are already being spoken while the rest is still arriving. Waiting
//! for a finished answer is what makes an assistant sound like a form submission.
//!
//! **Stoppability.** Every engine here ends up as a child process we own, so
//! barge-in is a kill: the instant the user starts talking, the voice stops
//! mid-word. An engine that can only be asked to finish politely cannot be
//! interrupted, and a voice you cannot interrupt is a broadcast, not a
//! conversation.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use serde_json::json;

#[cfg(test)]
mod tests;

mod system_command;
mod engine_status;
mod next_chunk;
mod speaker;

pub use system_command::*;
pub use engine_status::*;
pub use next_chunk::*;
