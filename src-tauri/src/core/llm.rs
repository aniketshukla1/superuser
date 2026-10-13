//! One model client — the request, the policy, and the reply, in one place.
//!
//! HARNESS-PLAN Phase 2. There were four hand-rolled Anthropic clients: the computer-use
//! loop, the MCP agent, the assistant loop, and `model_router`. Each built its own body,
//! set its own headers, mapped its own errors, and — this is the part that costs — grew
//! its own fixes. §2.3's usage accounting was added to one branch and not the other.
//! §2.5's cancel race was found, measured ("~25s once, which reads as a dead button"),
//! written down, and installed in one place. §2.1's thinking shape was wrong in all
//! four at once and had to be corrected in each.
//!
//! So this is not tidying. It is the phase that makes the later ones a change in one
//! file rather than four: prompt caching, context editing, task budgets and parallel
//! tools each land here once.
//!
//! What this module owns:
//!
//! * the request shape, including the per-model thinking/effort decision (`core::effort`)
//! * the retry policy (`core::retry`) — 429 and 529 are routine and must not cost a turn
//! * the cancel race, so Stop lands in under a second rather than at the end of the call
//! * usage, including the cache counters, for every caller rather than one
//!
//! What it deliberately does NOT own: what to do with the reply. Walking content blocks,
//! dispatching tools, and deciding when a turn is finished belong to the loop that asked.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use serde_json::{json, Value};

use crate::core::retry::{self, Next};
use crate::core::sessions::TokenUsage;

#[cfg(test)]
mod tests;

mod llm_request;
mod call_at;
mod sse_accum;

pub use llm_request::*;
pub use call_at::*;
use sse_accum::*;
