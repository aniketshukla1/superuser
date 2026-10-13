//! What must not leave in a tool call's arguments.
//!
//! A connected app's tool takes whatever the model puts in it, and the model has spent the
//! turn reading files, pages and replies. The failure this stops is not malice: it is a
//! model that read a `.env` while looking for a config value and then quoted the whole
//! block into a Notion page, or pasted a stack trace with an `Authorization:` header in it
//! into an issue. Once that has left the machine it cannot be recalled.
//!
//! **A finding says the KIND and WHERE, never the value.** `["notes", "body"] — a private
//! key` is enough to decide with, and is safe to write into the trail, put on a card, and
//! read out. Printing the secret to explain that a secret was about to leak is the same
//! leak with better manners.
//!
//! **Deliberately not general PII.** A scanner that promises to catch names, addresses and
//! phone numbers catches some of them, and the promise is what does the damage: people
//! stop reading what they are sending. Credentials are mechanical — a shape, not a
//! judgement — and that is the whole of what this claims.

use serde_json::Value;

#[cfg(test)]
mod tests;

mod walk;
mod endpoint_problem;

pub use walk::*;
pub use endpoint_problem::*;
