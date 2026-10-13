//! Handing work to another assistant.
//!
//! One assistant asks another for a piece of work and relays the answer back. It is a
//! delegated run of that assistant — its own prompt, tools, connected apps, model and
//! permission mode — in a conversation of its own that the person can open afterwards.
//!
//! **What it takes is typed**, which is the one place this departs from the obvious
//! build. The natural shape is `ask_coworker(who, message)`, and free text is the
//! commonest way a handoff goes quietly wrong: the assistant on the other end infers the
//! intent, re-derives the constraints, guesses what shape of answer was wanted — and when
//! it guesses wrong it does not fail, it answers something else confidently. Naming the
//! parts costs the asking model a little effort and removes most of that.
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
