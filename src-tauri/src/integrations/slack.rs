//! Slack connector — read channels, sync messages into memory, and take
//! **@mentions as work**: mention the bot in a channel and an assistant session
//! runs on your desktop, with the answer posted back in-thread.
//!
//! Inbound uses Web-API polling (`conversations.history`) rather than Socket
//! Mode: it needs only the bot token, no websocket dependency and no public
//! webhook, at the cost of ~poll-interval latency. Socket Mode is the real-time
//! upgrade path.

use super::SyncOutcome;
use crate::core::memory::{MemoryEngine, MemoryInput, Origin};
use crate::core::secrets;
use serde_json::Value;
use std::path::Path;

#[cfg(test)]
mod tests;

mod poll_mentions;

pub use poll_mentions::*;
