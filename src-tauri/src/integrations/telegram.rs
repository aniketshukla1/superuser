//! Two-way Telegram: messages come *in* as well as out.
//!
//! Outbound already existed — the app could push a reminder to your phone. The
//! missing half is being able to answer from your phone, and to see that exchange
//! back in the app rather than having it live only in Telegram.
//!
//! The security shape matters here more than the plumbing. A Telegram bot can be
//! messaged by anyone who finds its handle, and this one is wired to an agent with
//! file, shell and connector access. So the rule is an **allowlist, not a
//! blocklist**: only chats the user has explicitly named are served, and every
//! other message is dropped. An empty allowlist serves nobody — a bot token with no
//! configured chat is inert rather than open to the world.

use std::path::Path;

use serde::Serialize;

use crate::core::secrets;

#[cfg(test)]
mod tests;

mod poll;

pub use poll::*;
