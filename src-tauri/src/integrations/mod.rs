//! Integration registry. In the full design each integration is a local MCP
//! server (see `mcp` module); this registry tracks their connection state and
//! how many memories each has contributed to the Memory Tree.

pub mod descriptors;
pub mod rest;
pub mod oauth2;
pub mod accounts;
pub mod email;
pub mod mail;
pub mod slack;
pub mod telegram;
pub mod github;
pub mod google;
pub mod linear;
pub mod notion;
pub mod stripe;
pub mod prometheus;

use crate::core::memory::MemoryEngine;
use crate::core::secrets;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

#[cfg(test)]
mod tests;

mod sync_inner;
mod registry;

pub use sync_inner::*;
