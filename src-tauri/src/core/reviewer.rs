//! The auto-approve reviewer: a second model call that checks ONE proposed action
//! against what the user actually asked for, so routine steps go ahead without a card
//! and only doubtful ones stop to ask.
//!
//! Four rules hold it together. Each is enforced here or at its single call site, and
//! each guards against a mistake the straightforward version would make:
//!
//! 1. **It can upgrade "ask the person" to "proceed", and nothing else; a block stays a
//!    block.** It only ever sees a [`Decision::Ask`], after the floor
//!    ([`cmdgate::hard_deny`]) and the mode ([`inbox::decide`]) have ruled. A `Deny`
//!    never reaches it: a model that could argue its way past a refusal would be a
//!    bypass, not a reviewer.
//!
//! 2. **One action per request.** A turn that proposes several calls gets several
//!    reviews, each about a single action, so no verdict can be applied to the wrong
//!    one. Reviewing a batch would mean matching verdicts back to actions, and getting
//!    that wrong means approving the wrong thing.
//!
//! 3. **Failures close.** Bad JSON, an unknown verdict, an empty reply, a timeout, no
//!    model set up, a provider error: each becomes [`Verdict::Unsure`] and the person
//!    decides. Nothing in this file parses its way to running an action. A reviewer that
//!    failed open would turn any outage into silent approval of everything, just when
//!    nobody is looking.
//!
//! 4. **It sees no untrusted content.** Its input is the instructions, the user's own
//!    words and the proposed action; page text, file contents, email bodies, connector
//!    results and scanner output are never included. An attacker may be able to talk to
//!    the agent, but must never be able to talk to the judge. That is why [`Request`]
//!    has no field for tool output, and why the call site passes `ctx.prompt` instead of
//!    the transcript.

use std::time::Duration;

use serde_json::Value;

use crate::core::model_router::{self, ChatMessage};
use crate::core::settings::Settings;

#[cfg(test)]
mod tests;

mod instructions;
mod review;

pub use instructions::*;
pub use review::*;
