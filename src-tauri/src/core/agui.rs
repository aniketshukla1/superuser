//! Speaking AG-UI.
//!
//! AG-UI is the protocol every CopilotKit surface talks to an agent
//! over: a POST carrying the conversation, and a stream of typed events back. Speaking it
//! means a Superuser assistant — its prompt, its tools, its boundaries, its trail — can be
//! the agent behind somebody else's chat window, with none of this app's own UI involved.
//!
//! This module is the translation and nothing else: our run events in, AG-UI events out,
//! as values. The route that streams them is in `api_server`, and keeping the mapping
//! here is what lets it be tested without a server, a model or a socket.
//!
//! **What it does not do is run their tools.** AG-UI lets a surface offer tools that the
//! *surface* executes; ours run here, where the gateway, the boundaries and the approvals
//! are. So a tool call is reported as it happens — the surface can show the step — and
//! never handed over to be run somewhere this app cannot see.

use serde_json::{json, Value};

#[cfg(test)]
mod tests;

mod events_for;

pub use events_for::*;
