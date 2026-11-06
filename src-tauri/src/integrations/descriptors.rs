//! Connector descriptors: the data behind connecting and setting up an app. A new
//! connector is mostly a new entry here, not new code. Each entry gives the sign-in
//! method, the fields a person pastes in, the setup steps, and how the catalog shows
//! it (brand colour, aliases).
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
