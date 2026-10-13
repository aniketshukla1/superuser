//! The email bot — replies to the mail you allow, drafted for your approval.
//!
//! New mail from a sender you allow gets a reply drafted by the Email Assistant, and the draft
//! waits in the Inbox: nothing is sent until you approve it. Five rules make that safe to
//! leave running, because anyone can write you an email:
//!
//! * **Off by default, and an empty allowlist answers nobody.**
//! * **Only mail that arrives after it is on.** The first look reads nothing that was already
//!   there (`email::next_range`), so a backlog is never answered.
//! * **The assistant that drafts can read, not act.** An email is text a stranger chose. The
//!   Email Assistant runs in `discuss` mode with nothing that reaches outside — no web, no
//!   browser, no shell, no one to hand off to — so an email that says "run this" or "fetch that
//!   address" shapes a draft you read, and nothing else. If its page is changed so that it
//!   could act, the bot stops rather than drafting with it (`drafter_problem`).
//! * **The draft is parked by this code, never sent by the model.** A model asked to reply may
//!   call `send_message` itself, and a send can be let through by the reviewer or by an
//!   assistant in `auto`. Parking it here means every reply meets a person first.
//! * **It doesn't answer machines.** Mailing lists, automatic replies, bounces, no-reply
//!   senders, mail that failed DMARC, and your own mail are left alone (`screen`).

use serde_json::{json, Value};

use crate::core::personas::PersonaRegistry;
use crate::integrations::email::Inbound;

#[cfg(test)]
mod tests;

mod screen;

pub use screen::*;
