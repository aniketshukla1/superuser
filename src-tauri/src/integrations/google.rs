//! Real Google integration (Gmail + Calendar) via OAuth 2.0 with a local
//! loopback redirect + PKCE — the privacy-first desktop flow. The user supplies
//! their OWN OAuth client (Google Cloud Console → Desktop app), so no vendor
//! broker ever sees their data. Credentials + refresh token live in the OS
//! keychain as JSON under the "google" integration entry.

use super::SyncOutcome;
use crate::core::memory::{MemoryEngine, MemoryInput, Origin};
use crate::core::secrets;
use base64::Engine as _;
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::net::TcpListener;

mod oauth_connect;
mod sync;

pub use oauth_connect::*;
pub use sync::*;
