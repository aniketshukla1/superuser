//! Generic OAuth 2.0 connector engine (authorization code + PKCE over a loopback
//! redirect — the privacy-first desktop flow). Like `rest.rs`, connectors are
//! declared as *data*: an `OAuthSpec` carries the vendor's endpoints, scopes and
//! read feeds, and one implementation runs them all.
//!
//! **No broker.** The user registers an app with the vendor once and pastes its
//! client id (+ secret if the vendor requires one); the handshake then goes
//! straight from this machine to the vendor. A managed one-click flow would need
//! a hosted relay — that's the commercial tier, deliberately not faked here.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

use super::rest::Feed;
use super::SyncOutcome;
use crate::core::memory::MemoryEngine;
use crate::core::secrets;

#[cfg(test)]
mod tests;

mod specs;
mod stored_credentials;
mod helpers;
mod flow;

pub use specs::*;
pub use stored_credentials::*;
use helpers::*;
pub use flow::*;
