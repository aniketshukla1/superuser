//! Generic REST connector engine. Most connectors are the same shape — attach a
//! credential, GET a list endpoint, map JSON items into the Memory Tree — so they
//! are declared as *data* (`RestSpec`) and driven by one client, instead of one
//! bespoke file each. Connectors with a genuinely different protocol (GraphQL,
//! OAuth-heavy Google/Microsoft) keep their own module.
//!
//! Credentials: a connector with a single field stores the raw token; one with
//! several (email + token + site) stores a JSON object under the same key.

use super::SyncOutcome;
use crate::core::memory::{MemoryEngine, MemoryInput, Origin};
use serde_json::Value;

#[cfg(test)]
mod tests;

mod apply_auth;
mod save_items;
mod connector_specs;

pub use apply_auth::*;
pub use save_items::*;
pub use connector_specs::*;
