//! Local OpenAI-compatible HTTP API:
//! it lets *other* tools drive your assistants — point any OpenAI-compatible client
//! at `http://127.0.0.1:<port>/v1` and use a persona id as the "model".
//!
//! Security posture, deliberately conservative:
//! - **off by default**, enabled per-session from Settings;
//! - bound to **127.0.0.1 only** (never a public interface);
//! - guarded by a **token** generated at start and shown to the user, required in
//!   `Authorization: Bearer …`.
//! The point is local automation, not exposing your brain to the network.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::State as AxState;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};

#[cfg(test)]
mod tests;

/// The GUI↔server contract, checked mechanically.
///
/// `surfaces/gui` is a fork of another product's client, and it calls paths this
/// server never implemented. Nothing fails loudly when one is missing: `fetch`
/// gets a 404, the client's `.catch()` swallows it, and the card renders its empty
/// state — indistinguishable from "you have none of these yet". Four instances
/// were found by accident before anyone went looking; see `notes/reports/API-CONTRACT-AUDIT.md`.
///
/// So this is a ratchet, not a pass/fail: the gaps that exist are listed, and the
/// test fails when a NEW one appears. Deleting an entry from `KNOWN_GAPS` as it
/// gets served is the point; adding one should take an argument.
#[cfg(test)]
mod contract_tests;

mod chat_completions;
mod health;
mod is_running;
mod sessions_api;
mod personas_api;
mod memories;
mod journal_api;
mod knowledge_base;
mod workspaces;
mod remaining_gui_surfaces;
mod assistants_browsers;
mod session_socket;

pub use chat_completions::*;
pub use health::*;
pub(crate) use is_running::*;
use sessions_api::*;
use personas_api::*;
use memories::*;
use journal_api::*;
use knowledge_base::*;
use workspaces::*;
use remaining_gui_surfaces::*;
use assistants_browsers::*;
pub use session_socket::*;
