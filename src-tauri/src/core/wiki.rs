//! The wiki layer — what the assistant has learned about *how its own work goes*,
//! kept across every review pass whether or not that pass changed anything.
//!
//! There are already three stores of what the assistant knows, and none of them is
//! this one:
//!
//! | store | organized by | about | rolled back? |
//! |---|---|---|---|
//! | vault notes (`MemoryEngine`) | source | what arrived from somewhere | no |
//! | journal (`journal::Journal`) | time | what was established, and when | no |
//! | knowledge (`knowledge::Knowledge`) | topic | a subject the agent wrote up | no |
//! | **wiki (here)** | **failure mode** | **how the assistant's own work goes wrong, and what fixed it** | **never** |
//!
//! Before this existed, [`crate::core::evolution`] was amnesiac by construction. A
//! review saw one conversation, the current skills and the core memory — never its
//! own history. Two things followed, and both are fixed here:
//!
//! * **A pass that noticed something with no skill fix yet threw it away.**
//!   `Outcome::Silent` is the *designed-for* common case, so nearly everything a
//!   review ever noticed was discarded a token after it was thought.
//! * **An undone change was proposed again.** `restore()` puts the skill back and
//!   `forget_snapshot()` drops the record, leaving the next review with precisely
//!   the inputs that produced the edit the user had just rejected. It would write
//!   the same edit. Then the user would undo it again.
//!
//! So the wiki lives deliberately *outside* [`crate::core::evolution::Scope`]: an
//! undo puts the skill back and leaves the lesson standing — including the lesson
//! that this particular edit was not wanted. That asymmetry, skills roll back and
//! knowledge never does, is the whole point of the layer.
//!
//! Design from WikiSkill (Tang, Rashtchian, Ferng, Tomkins, Juan & Vu,
//! arXiv:2608.27454), which measures this layer as the difference between skill
//! evolution working and not working: 48.7% → 63.7% average across five benchmarks.
//! Its other load-bearing result is negative, and is honoured in `evolution` rather
//! than here — the wiki is shown to the *reviewer* only, never to the assistant
//! doing the work, because an assistant that can read the answers stops generating
//! the failures the reviewer needs to see.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::core::journal::Journal;
use crate::core::store::write_atomic;

#[cfg(test)]
mod tests;

mod kind;
mod wiki;
mod commit;

pub use kind::*;
pub use commit::*;
