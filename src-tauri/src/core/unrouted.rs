//! Inbound messages that reached this machine and were dropped.
//!
//! The allowlist that guards Telegram is doing exactly what it should: anyone can
//! message a bot they find, and this one answers with an agent that has file and
//! shell access, so a chat nobody vouched for gets no reply. What it did *not* do
//! was tell anyone. `telegram::poll` reached a message from an unlisted chat and
//! `continue`d, and that was the end of it — the sender waits forever, and the
//! person who owns the bot never learns that someone tried.
//!
//! Silence is the right answer to the stranger. It is the wrong answer to the
//! owner. So the drop is recorded here and shown in the Inbox, where the user can
//! see who tried, what they said, and why it went nowhere — and then decide,
//! deliberately, whether to add them to the allowlist.
//!
//! # This content reaches the agent by exactly one path, and a person opens it
//!
//! Every entry here is text written by someone who is, by construction, not
//! trusted — we dropped their message precisely because nobody vouched for them.
//! It exists to be *read by the user in the UI*. Nothing in this module is wired
//! into recall, context, or any tool result, and none of it may become one: a queue
//! of attacker-supplied text with an automatic path into the model's context would
//! be a better prompt-injection channel than any of the ones the egress spine and
//! the taint tracking were built to close.
//!
//! The single exception is `allow_deliver`, and it is not really an exception. The
//! user has read the message on screen, decided the sender may drive an agent with
//! file and shell access, and asked for that one message to be answered. Delivering
//! it grants nothing the allowlist entry did not just grant — the same sender can
//! send the same text again a second later and it will go straight through. What
//! must never happen is delivery without that click, so `take` removes the item and
//! hands it back exactly once, and no code here calls it.
//!
//! The store is a bounded ring. An unlisted chat can send as fast as it likes, and
//! an unbounded log of a stranger's text is a disk-filling primitive.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;

mod unrouted;

pub use unrouted::*;
