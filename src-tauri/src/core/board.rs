//! The work board — what a lead assistant decomposed, who is doing it, and who
//! checked it.
//!
//! A todo list (`todo_write`) is the right shape for one turn: it is the progress
//! panel, it lives in memory, and it dies with the run. This is the other thing —
//! work that outlives a turn, is picked up by someone other than whoever wrote it
//! down, and is *verified* before it counts as finished.
//!
//! # Why an event log rather than a table of rows
//!
//! The board is a projection of an append-only log, which costs a fold on every
//! read and buys the only thing that makes the rule below enforceable: the log
//! remembers **who did what**. A table of current state can say an item is done.
//! It cannot say who built it and who passed it, and those two being different
//! people is the entire point.
//!
//! # The rule
//!
//! **A builder never grades its own work.** An item reaches [`ItemState::Done`]
//! only through a passing review by an actor who is not the one that moved it to
//! review. This is checked here, in code, rather than asked for in a prompt —
//! because a model marking its own work complete is not a rare failure, it is the
//! default behaviour of anything optimising to finish.
//!
//! The actor is the run's session id, which the platform assigns: a lead is
//! `abc`, the workers it spawns are `abc-sub1`, `abc-sub2` (see
//! `commands::spawn_subagent`). A model cannot choose or forge it, which is what
//! makes the check worth having. An honesty note about the boundary: a lead
//! running everything in its own turn is one actor, so it cannot review its own
//! items and will be told so — that refusal is the feature.
//!
//! Boards are per-project, keyed by workspace path, so work follows the codebase
//! rather than the conversation.

use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;

mod item_state;
mod board;

pub use item_state::*;
