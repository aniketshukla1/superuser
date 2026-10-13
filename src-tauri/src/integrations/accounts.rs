//! Multi-account connectors — several logins for the same app (two Gmail
//! addresses, two Slack workspaces, a personal and a work GitHub), with one
//! marked default.
//!
//! Storage: the registry (labels + which is default) lives under
//! `accounts_<connector>`; each account's credential under
//! `integration_<connector>__<account>`. A connector connected before this
//! existed keeps its credential at `integration_<connector>` and is surfaced as
//! the implicit "default" account, so nothing breaks.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::core::secrets;

#[cfg(test)]
mod tests;

mod registry;
mod secrets_backed_wrappers;

pub use registry::*;
pub use secrets_backed_wrappers::*;
