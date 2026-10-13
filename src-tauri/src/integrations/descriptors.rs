//! Connector descriptors — the data that drives connect/setup. Adding a connector
//! is (mostly) data,
//! not code. A descriptor declares its auth method, the fields the user pastes,
//! step-by-step instructions, and its catalog metadata (brand color, aliases).
//!
//! Auth kinds:
//! - `api_token` / `token` / `bot_token` — the user pastes a credential (private,
//!   no broker: the token goes straight from this machine to the vendor).
//! - `oauth` — needs an OAuth handshake. Local-first path is a pasted access token
//!   or a self-registered app; a managed one-click flow is the commercial tier.
//! - `app_password` — IMAP-style host + address + app password.
//! - `none` — reachable without stored credentials (or not yet wired).
//!
//! `implemented` marks connectors whose sync/tools actually run today; the rest are
//! catalogued (so the UI can show the full surface honestly) pending their client.

#[cfg(test)]
mod tests;

mod connectors;
mod list_info;

pub use connectors::*;
pub use list_info::*;
