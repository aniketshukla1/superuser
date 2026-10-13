//! Boundaries — what the user's assistants may never do, in rules the user can read.
//!
//! The approval engine decides *how* an action is allowed to happen
//! — ask first, run, park it for later — and it has always been expressed in modes
//! and risk classes, which are the app's words rather than the user's. A boundary is
//! the user's own sentence: "never send a message", "nothing touches a .env file",
//! "once a run has read the web it does not run commands". Written as a rule
//! (`core::rules`), checked by `core::gateway` on every tool call, before the
//! approval engine ever sees it.
//!
//! # The semantics
//!
//! * **Deny is checked first, and a deny wins.** A rule that removes permission must
//!   never be defeated by a broader rule that grants it, or nobody can reason about
//!   what they have forbidden.
//! * **A broken deny rule denies.** One typo refuses the action rather than admitting
//!   it: the failure is loud, immediate and safe, and the alternative is a user who
//!   believes they have forbidden something they have not. The Boundaries page checks
//!   every rule before it is saved, so this is the backstop, not the experience.
//! * **A broken allow rule permits nothing, and an empty allow list permits nothing.**
//!   The default allow list is `true` — anything not refused above — and it says so.
//! * **`record` mode decides and writes it down, then lets the action through.** "A
//!   governance feature nobody dares switch on is not a governance feature": a rule
//!   can be watched against real work before it refuses any.
//!
//! # What this adds to, and what it does not replace
//!
//! Boundaries are additive. The approval modes still ask; the command floor still
//! refuses `rm -rf /` before anyone is asked; local-only still stops the network. A
//! boundary can only take permission away.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::core::audit::{Decision, Entry, Initiator, Subject};
use crate::core::rules::{self, Program};

#[cfg(test)]
mod tests;

mod presets;
mod load;
mod loaded;
mod testing_rule_against;

pub use presets::*;
pub use load::*;
pub use loaded::*;
pub use testing_rule_against::*;
