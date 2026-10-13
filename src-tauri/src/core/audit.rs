//! Audit log — an append-only record of everything that leaves this machine or
//! changes something outside it: connector syncs, OAuth connections, messages
//! sent, shell commands, approvals.
//!
//! Stored as JSONL in the app data dir so it's greppable and can't be silently
//! rewritten; reads return the most recent entries first.
//!
//! # Version 2
//!
//! The first version was a row of `{at, kind, actor, target, detail, ok}` written after
//! some actions and not others, and it could not answer the questions an audit trail
//! exists for. Four were added, and they are the reason
//! the trail reads as trustworthy:
//!
//! * **What was decided, by which rule, and whether it was carried out.** A row for an
//!   action carries a [`Decision`], written by `core::gateway` *before* the action runs.
//!   "An action that was not recorded did not happen, because there is no path that
//!   acts without writing the row first." A failure, a stop, a parked approval or a
//!   refusal by the user adds a second row after it.
//! * **What started the run.** An [`Initiator`] — a person, a schedule, a watch, a
//!   message from outside, a subagent, the API. "Nobody watching" is a filter over it.
//! * **What is written down, and what never is.** A command in full, its output never;
//!   a file's path, never its contents; a message's length, not its words; a URL
//!   without its query. Anything shaped like a credential is replaced before a row is
//!   written ([`scrub`]), so the trail is safe to read out and to share.
//! * **Whether it has been edited.** Each row carries the hash of the one before it,
//!   and `audit.head` records the newest. [`verify`] walks the chain and says in words
//!   where it breaks. It cannot stop someone with the file from rewriting it — nothing
//!   on the user's own machine can — but an edit, an insertion or a removal no longer
//!   passes silently.
//!
//! Rows written before version 2 still read: every new field is optional, and a row
//! with no hash is reported as "from before the trail was linked", not as a break.
//!
//! The file rotates monthly (`audit-2026-08.jsonl`), and the chain runs across the
//! rotation. Retention is off by default and a setting — deleting somebody's audit
//! trail because a default said so is the worse failure.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[cfg(test)]
mod tests;

mod subject;
mod append;
mod verify;
mod recording_rules;

pub use subject::*;
pub use append::*;
pub use verify::*;
pub use recording_rules::*;
