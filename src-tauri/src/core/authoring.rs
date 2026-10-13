//! Making a skill or an assistant by talking to one.
//!
//! Two built-in skills (`skill-creator`, `persona-creator`) do the interviewing; these are
//! the tools they use. Looking is free — list and read, so a model can find out whether the
//! thing already exists before proposing another one. Saving is a card the person presses:
//! an assistant that can mint an assistant, or write instructions every later run obeys,
//! without anyone agreeing to it has been handed the keys to the app.
//!
//! **Problems go back to the model, not to the person.** A half-written draft is the
//! model's to fix; handing "default_permission_mode must be one of …" to someone who never
//! saw the draft asks them to debug something they have not read. So every check here
//! answers in sentences addressed to the author, and the person only ever sees a finished
//! proposal to accept or refuse.

use std::path::{Path, PathBuf};

use super::personas::{self, PersonaRegistry, GRANTABLE_TOOLS};
use super::skills::SkillStore;

#[cfg(test)]
mod tests;

mod check_persona;

pub use check_persona::*;
