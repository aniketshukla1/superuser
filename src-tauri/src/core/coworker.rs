//! Handing work to another assistant.
//!
//! One assistant asks another for a piece of work and relays the answer back. It is a
//! delegated run of that assistant — its own prompt, tools, connected apps, model and
//! permission mode — in a conversation of its own that the person can open afterwards.
//!
//! **The request has fields.** The obvious design would be `ask_coworker(who, message)`,
//! but a free-text handoff fails without anyone noticing: the receiving assistant has to
//! guess the goal, the limits and the kind of answer wanted, and a wrong guess doesn't
//! error, it produces a confident answer to a different question. Filling in named fields
//! is a little more work for the asking model and prevents most of that.
//!
//! **Nothing here widens what a run may do.** The asked assistant runs under the stricter
//! of the two permission modes, inherits the asker's taint, and is unattended — nobody is
//! watching *its* turn, whoever is watching the one that asked. A chain that could act
//! more freely than the run that started it would make delegation the way around every
//! approval in the app.

use std::collections::HashMap;

use serde_json::Value;

use super::personas::PersonaRegistry;

#[cfg(test)]
mod tests;

mod resolve;
mod cap_refusal;
mod mention;

pub use resolve::*;
pub use cap_refusal::*;
pub use mention::*;
