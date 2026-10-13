//! The auto-approve reviewer — a second model call that judges ONE proposed
//! action against what the user actually asked for, so routine work runs without
//! a card and only the questionable actions interrupt.
//!
//! Four invariants. Each is enforced here or at the one call site, and each exists
//! because the obvious implementation gets it wrong:
//!
//! 1. **It can only turn "ask the human" into "go ahead" — never "blocked" into
//!    "go ahead".** It is consulted exclusively on [`Decision::Ask`], after the
//!    floor ([`cmdgate::hard_deny`]) and after the mode
//!    ([`inbox::decide`]) have had their say. A `Deny` never reaches it. A model
//!    that can talk its way past a refusal is not a reviewer, it is a bypass.
//!
//! 2. **One action per request.** A turn proposing several calls makes several
//!    reviewer calls, each carrying exactly one action, so a verdict physically
//!    cannot land on the wrong action. Batch a list and you have created a
//!    re-pairing problem whose failure mode is approving the wrong thing.
//!
//! 3. **Fail closed.** Malformed JSON, an unknown verdict, an empty reply, a
//!    timeout, no model configured, a provider error — all become [`Verdict::Unsure`],
//!    and the human decides. There is no parse path in this file that results in
//!    execution. A reviewer that fails open converts every outage into blanket
//!    auto-approval, silently, at exactly the moment nobody is watching.
//!
//! 4. **It never reads untrusted content.** Its input is the instructions, the
//!    user's own words, and the proposed action. Page text, file contents, mail
//!    bodies, connector results and scanner output never appear. The attacker can
//!    address the agent; they must never be able to address the judge. This is
//!    why [`Request`] has no field for tool output and why the call site passes
//!    `ctx.prompt` rather than the transcript.

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
