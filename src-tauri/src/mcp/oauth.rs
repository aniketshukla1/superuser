//! OAuth 2.1 for remote MCP servers, per the MCP authorization spec:
//! 1. hit the MCP endpoint unauthenticated → 401 with the protected-resource
//!    metadata location,
//! 2. discover the authorization server + its endpoints,
//! 3. dynamically register a client (falling back to a user-supplied client id),
//! 4. authorization-code + PKCE via a local loopback redirect,
//! 5. store tokens; refresh on demand.

use crate::core::secrets;
use base64::Engine as _;
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::net::TcpListener;

mod discover;
mod connect;

pub use discover::*;
pub use connect::*;
