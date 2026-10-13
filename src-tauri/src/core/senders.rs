//! Senders — outbound messaging for assistants. Every send is approval-gated by
//! `core::inbox` before it reaches here; this module only performs the delivery.
//!
//! A target is `channel:address` (e.g. `telegram:12345`, `slack:#ops`). A bare
//! address uses the user's default channel.

use crate::core::secrets;

#[cfg(test)]
mod tests;

mod send_whatsapp;

pub use send_whatsapp::*;
