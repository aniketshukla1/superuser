//! Choosing which assistant a message is for.
//!
//! A message that names nobody still has to go somewhere. This picks, and the rules it
//! picks by are ordered so that the cheap certain ones come first:
//!
//! 1. **One candidate** — nothing to choose.
//! 2. **The only one that can reach a system the message names.** "Refund the Stripe
//!    payment" belongs with whoever holds Stripe, and if exactly one assistant does, no
//!    model is needed to say so.
//! 3. **A small model**, over the names and taglines, answering with an id and how sure it
//!    is. Below [`FLOOR`] the answer is treated as no answer.
//!
//! **It never throws and it never picks nobody.** Every way of not knowing ends at the
//! default assistant with a reason, and the reason is recorded — a router that silently
//! sends work to the wrong desk is worse than one that says it guessed.

use super::model_router::{self, ChatMessage, Workload};
use super::personas::{PersonaListItem, PersonaRegistry};
use super::settings::Settings;

#[cfg(test)]
mod tests;

mod pick;

pub use pick::*;
