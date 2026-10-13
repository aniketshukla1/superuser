//! Asking the user a question mid-run, and actually waiting for the answer.
//!
//! `ask_user` has been in the catalogue from the start, is granted to every
//! assistant by `agent::ALWAYS_ON`, and has never once reached a human. The tool
//! returned the string `[awaiting user] <question>` to the model and the loop
//! carried straight on — so an assistant that did the right thing, stopping to
//! check, was punished for it: it got no answer back, and its cheapest next move
//! was to guess. That is why charts appear as files nobody asked for.
//!
//! Both other halves were already built. The GUI has rendered the question card
//! since the Inbox landed (`question_requested` → `InboxItemCard`), and knows how
//! to send the answer (`question_response`). The session socket answered that
//! frame with "isn't handled by this build yet".
//!
//! This is the join: a rendezvous keyed by session id. The tool parks a receiver
//! and awaits it; whichever socket carries the user's answer resolves it. Kept
//! deliberately outside the socket and outside the run loop, because neither
//! outlives the other reliably — the user can switch chats mid-question and
//! answer from a socket that did not exist when the question was asked.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use serde::Serialize;
use serde_json::{json, Value};
use tokio::sync::oneshot;

#[cfg(test)]
mod tests;

mod form;
mod form_ask;
mod ask;

pub use form::*;
use form_ask::*;
pub use ask::*;
