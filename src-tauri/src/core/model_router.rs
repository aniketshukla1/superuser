//! Hybrid LLM router. Defaults to a local Ollama model for privacy; routes to
//! Anthropic Claude only when the user has explicitly enabled cloud AND is not
//! in local-only mode AND a key is present. Local-only is a hard egress switch.

use super::providers;
use super::secrets;
use super::settings::Settings;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[cfg(test)]
mod tests;

mod ollama_model;
mod openai_compatible;
mod claude;

pub use ollama_model::*;
pub use openai_compatible::*;
use claude::*;
