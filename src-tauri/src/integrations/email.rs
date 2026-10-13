//! Universal email connector — IMAP for reading, SMTP for sending. Works with any
//! provider (Fastmail, iCloud, a company mail server, Gmail with an app password)
//! without OAuth or a broker: credentials go straight from this machine to your
//! mail host.
//!
//! Credentials are stored as a field map: address, app_password, imap_host,
//! optional smtp_host (defaults to the IMAP host with the usual submission port).

use super::SyncOutcome;
use crate::core::memory::{MemoryEngine, MemoryInput, Origin};
use crate::core::secrets;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};

#[cfg(test)]
mod tests;

mod sync;
mod email_bot_reading;

pub use sync::*;
pub use email_bot_reading::*;
