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
//! * **Deny rules run first, and any deny wins.** If a broader allow could override a
//!   rule that takes permission away, nobody could be sure what they had forbidden.
//! * **A deny rule that fails to evaluate refuses.** A typo blocks the action instead of
//!   letting it pass, so the mistake shows at once and costs nothing; the other way round,
//!   the person would think something was forbidden when it wasn't. The Boundaries page
//!   checks each rule before saving it, so this is the safety net, not the usual path.
//! * **A broken allow rule permits nothing, and an empty allow list permits nothing.**
//!   The default allow list is `true` — anything not refused above — and it says so.
//! * **`record` mode decides and writes it down, then lets the action through.** People
//!   only switch on a control they can try safely, so a rule can be watched against real
//!   work before it refuses anything.
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
