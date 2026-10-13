//! What a turn actually did, kept so the next turn can see it.
//!
//! HARNESS-PLAN H2, and the largest behavioural gap in the harness. Inside a turn
//! `messages` accumulates properly — tool calls, their results, the model's own
//! reasoning about them. At the turn boundary all of it is thrown away and history
//! replays as `{role, content}` pairs of final prose. So on turn 2 the model cannot
//! see that on turn 1 it read the file, ran the command, got the error and worked
//! around it. It re-derives, re-reads, re-runs.
//!
//! **The plan's premise for fixing this was wrong and the correction is the work.**
//! It says the rich record is already on disk in `Message.items` and replay simply
//! does not read it. `items` holds `kind`-tagged *UI* objects — todos, charts,
//! spawned threads, a scheduled-run marker — and nothing has ever written a wire
//! transcript into it. The record was not on disk at all; it was discarded when
//! `run_agent` returned. So this module both captures it and replays it.
//!
//! ## Why replay is not simply "hand the messages back"
//!
//! The three wire shapes this app speaks are not interchangeable, and the mismatches
//! are all silent-until-they-are-not:
//!
//! | | assistant call | result message |
//! |---|---|---|
//! | Claude | `content: [{type: "tool_use", id, name, input}]` | `{role: "user", content: [{type: "tool_result", tool_use_id, …}]}` |
//! | OpenAI | `tool_calls[].function.arguments` — a JSON **string** | `{role: "tool", tool_call_id, content}` |
//! | Ollama | `tool_calls[].function.arguments` — a JSON **object** | `{role: "tool", content}`, no id |
//!
//! Handing an Ollama-shaped transcript to OpenAI omits `tool_call_id`, which it
//! requires. Handing either to Anthropic is not even the same schema. So native
//! replay happens only into the shape that produced it, and every other combination
//! is *projected* to labelled text — lossy, and saying so, which is the difference
//! between a summary and a lie.
//!
//! ## Two invariants that fail loudly if this gets them wrong
//!
//! Both APIs require every tool call to be answered. A turn that ended at the step
//! limit, or on Stop, or on an error can leave a trailing call with no result — and
//! replaying that is a 400 on the *next* turn, for a mistake made in the previous
//! one. [`TurnWire::repair`] drops those trailing calls before anything is stored.

use crate::core::model_profile::ModelProfile;
use crate::core::providers::ProviderKind;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[cfg(test)]
mod tests;

mod turn_wire;
mod turn_wire_transcript;
mod replay;

pub use turn_wire::*;
pub use turn_wire_transcript::*;
pub use replay::*;
