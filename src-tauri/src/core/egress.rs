//! The egress spine — the one place a request can leave this machine.
//!
//! The product's first claim is that your data stays on your Mac. We had the
//! parts to back that up and no spine: [`webguard`](super::webguard) blocks SSRF,
//! [`privacy`](super::privacy) redacts, [`audit`](super::audit) records decisions,
//! `local_only` is a switch, taint tracks provenance. What nothing could answer is
//! the question the claim actually rests on: **what left this machine, where to,
//! and why.**
//!
//! Before this module there were fifty `reqwest::Client::new()` calls across
//! twenty files, and two consequences followed from that. There was no record of
//! outbound traffic at all — the Activity page's own tooltip said "a record of
//! everything that left this machine", which was not true of the action log it
//! showed. And `local_only`, documented in `model_router.rs` as "a hard egress
//! switch", was enforced on model calls, the two web tools and web search, and
//! nowhere else: a connector sync in local-only mode still called GitHub, MCP
//! still dialled out, Slack still delivered.
//!
//! ```text
//!   caller ─► egress::client("github")   the only way to build a client
//!                     │
//!                     ├─ local host?  ─► allowed always, never recorded
//!                     │                  (it didn't leave the machine)
//!                     ├─ local_only?  ─► refused, and the refusal is recorded
//!                     └─ otherwise    ─► sent, and recorded: host, path, purpose
//! ```
//!
//! # Why this is a type and not a convention
//!
//! A helper everyone is *supposed* to use is not a chokepoint; it's a habit, and
//! the first call site that forgets makes the log quietly incomplete. An
//! incomplete egress log is worse than none, because it is read as complete. So
//! the enforcement is a test — [`tests::every_outbound_request_goes_through_here`]
//! greps the source and fails if `reqwest::Client` is built anywhere but here.
//! That is what keeps the claim true after the next feature lands.
//!
//! # What is recorded, and what deliberately isn't
//!
//! Host, path, method, purpose, bytes sent, status. **Never the query string** —
//! query strings carry tokens and search terms, and a privacy log that quietly
//! becomes a second copy of your secrets has defeated itself. Never the body.
//!
//! Requests to loopback and private addresses aren't recorded at all. Not an
//! oversight: this log answers "what left this machine", and Ollama's every token
//! call to `127.0.0.1` didn't. Logging them would bury the handful of lines that
//! matter under thousands that don't.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

macro_rules! forward {
    ($name:ident, $($arg:ident: $ty:ty),*) => {
        pub fn $name(self, $($arg: $ty),*) -> Self {
            Self { inner: self.inner.$name($($arg),*), ..self }
        }
    };
}

#[cfg(test)]
mod tests;

mod now;
mod policy;
mod record;
mod client;

use now::*;
pub use policy::*;
pub use record::*;
pub use client::*;
