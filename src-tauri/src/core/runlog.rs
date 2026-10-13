//! What a run actually did, kept after the run is over.
//!
//! A turn already broadcasts its steps to whoever is attached, and buffers them so
//! reattaching mid-turn replays what you missed. Both of those live in memory and
//! are dropped the moment the turn ends. That is fine for a conversation you are
//! watching and useless for everything else: an automation that ran at 7am, a
//! subagent that failed, a background run that produced an answer you now doubt.
//! The transcript keeps the *answer*; nothing kept the working.
//!
//! So every frame is also appended here, as JSONL beside the run's own folder.
//! Three properties are deliberate:
//!
//! * **Addressed by offset.** A frame's sequence number is its line number, so a
//!   reader can ask for "everything after 40" and get a stable answer whether the
//!   run is still going or finished last week.
//! * **Redacted before it is written.** Credentials are masked on the way to disk
//!   rather than on the way out, because the risk is the file existing at all. PII
//!   is deliberately *not* stripped — see [`crate::core::privacy::redact_secrets`].
//! * **Structure, not tokens.** Streaming deltas are skipped. They would be
//!   thousands of writes per turn to reconstruct text the final message already
//!   holds, and a journal of a run should read like an account of what happened.
//!
//! Writes are best-effort: a journal that fails must never fail the turn it is
//! describing.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

#[cfg(test)]
mod tests;

mod read;

pub use read::*;
