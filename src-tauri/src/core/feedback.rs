//! What the user thought — the only judgement in this app that is not a model's.
//!
//! # Why this exists, and why it is not a rubric grader
//!
//! `harness_eval` is deliberately judge-free: every check is a pure function over a
//! record, because a model grading a model would make the scoreboard's own
//! reliability the thing in question. That rule cost real coverage. Two failure
//! modes the 2026 literature names could not be added for exactly that reason —
//! tool-output hallucination ("the answer cites a result the tool never returned")
//! and, one store over, an answer that quietly prefers a superseded memory. Both
//! need judgement about *meaning*, which no `contains` can supply.
//!
//! A person supplies it without the circularity. This is the exogenous acceptance
//! signal *Self-Authored Verification Is Unreliable* (2607.24300) asks for: the
//! agent may not write it, may not score it, and cannot argue with it.
//!
//! # A bare thumbs-down is nearly worthless, so this does not collect one
//!
//! "Bad" is not a work queue. It says a turn was wrong and nothing about which part
//! of the harness was wrong, and a hundred of them still say nothing — which is how
//! feedback buttons end up as a number nobody can act on.
//!
//! So a downvote arrives with a [`Reason`] chosen from a short fixed list, in the
//! same discipline `ask_user` is held to: a question with no options is refused,
//! because writing the options is the work. Each reason names a part of this system
//! that could be at fault, so the fold in [`Report`] points somewhere.
//!
//! And every record carries the **evidence** the turn already produced — model,
//! persona, tools called, whether recall ran, steps, seconds — captured by us rather
//! than typed by the user. That is the half that turns "this was wrong" into "this
//! was wrong and `recall_memory` ran and returned nothing", which is a bug report.
//!
//! # What it is not allowed to do
//!
//! Nothing here feeds a model automatically. It is a store and a report; acting on
//! it is a person's decision, and `evolution.rs`'s allowlist does not reach the
//! scoreboard for the same reason it never did.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;

mod reason;
mod report;

pub use reason::*;
pub use report::*;
