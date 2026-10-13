//! Ollama helpers: discover installed models, resolve a usable chat model, and
//! report status. Pure helpers are unit-tested; network calls are thin wrappers.

use super::settings::Settings;
use serde::Serialize;
use serde_json::Value;

#[cfg(test)]
mod tests;

mod model_info;
mod parse_model_infos;

pub use model_info::*;
pub use parse_model_infos::*;
