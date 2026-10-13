//! One loop, shaped to the model running it.
//!
//! HARNESS-PLAN H6: 53 tool schemas, 24 steps and a ~9k-token prompt go out
//! identically whether the model is a 3B local one or a frontier model. The only
//! adaptation was `underpowered`, a proxy that treats a 70B and a 1B alike and whose
//! whole effect is to add a disclaimer paragraph. Small models do not need a warning
//! label; they need fewer tools, shorter prompts, tighter steps and stricter
//! argument checking.
//!
//! Phase 2 made `ModelProfile.tier` available and nothing read it. This is its reader.
//!
//! ## What narrowing must not be
//!
//! Sending a small model fewer tools is a capability cut unless it can get them back.
//! Anthropic has a native answer — `defer_loading` plus the tool-search tool, which
//! Phase 6 already uses — and Ollama and the OpenAI-compatible providers have none. So
//! this module ships the same idea by hand: withheld tools stay withheld until
//! [`find_tools`] asks for them, and the loop adds what it finds to the wire for every
//! later step of the turn. Nothing is lost; it is deferred, on providers that have no
//! word for deferral.
//!
//! ## Why a token budget rather than a list of names
//!
//! Because the thing being spent is tokens, and because a per-tier name list goes
//! stale the moment a capability is added. The priority order is the one policy here,
//! and it is anchored on `agent::RESIDENT` — the list Phase 6 already reviewed and
//! measured — rather than invented again.

use crate::core::agent::{self, ToolSpec};
use crate::core::model_profile::{ModelProfile, Tier, ToolSupport};
use serde_json::{json, Value};

#[cfg(test)]
mod tests;

mod eval_tool_budget;
mod loop_shape;
mod step_notice;
mod check_args;

pub use eval_tool_budget::*;
pub use step_notice::*;
pub use check_args::*;
