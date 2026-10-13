//! What the model in front of us can actually do — as data, not as folklore.
//!
//! HARNESS-PLAN H1: the loop asks its questions by branching on vendor identity
//! (`if openai`, `if !Anthropic`, `let can_think = !openai && model_can_think`)
//! because `Provider` has no field for context window, tool support, vision,
//! thinking, usage reporting or caching. Every one of those is a property of the
//! *model*, and every one of them is discoverable. This module is where the loop
//! asks once and gets an answer.
//!
//! ## Where the answers come from, best source first
//!
//! | Kind | Source | Gives |
//! |---|---|---|
//! | Ollama | `/api/tags` | `details.context_length`, `capabilities[]` — the real window and the real capability list |
//! | Anthropic | `GET /v1/models` | `max_input_tokens`, `max_tokens`, `capabilities.{thinking,effort,image_input,pdf_input,structured_outputs,context_management}` |
//! | OpenAI-compatible | `GET /v1/models` + a handshake probe | existence, then measured tool-calling and usage reporting |
//! | Anything else | the shipped table, then the conservative default | enough to run a turn |
//!
//! [`ProfileSource`] is carried on every profile because a discovered capability
//! and a guessed one must never look the same — not to the code, and not to the
//! user reading the model picker.
//!
//! ## Two fields the plan lists that are deliberately absent
//!
//! * **`arg_delivery`** (`JsonString | Object`). The tool-argument parse site already
//!   handles both shapes polymorphically — `Value::String(s) => from_str(s)`, an object
//!   used as-is — because Ollama returns arguments as an object and OpenAI returns them
//!   as a string *on the same wire format*. A field restating that would have no reader.
//! * **`price`**. `core::pricing` owns the rate table and knows that a local model is
//!   free while an unlisted cloud model is *unknown*, which is a different fact.
//!   [`ModelProfile::price`] delegates there rather than keeping a second copy.

use crate::core::ollama::ModelInfo;
use crate::core::providers::{self, ProviderKind};
use crate::core::settings::Settings;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

#[cfg(test)]
mod tests;

mod model_profile;
mod parse_anthropic_model;
mod declared;
mod discover;

pub use model_profile::*;
pub use parse_anthropic_model::*;
pub use declared::*;
pub use discover::*;
