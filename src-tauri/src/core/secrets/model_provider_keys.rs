//! model provider keys (OpenAI-compatible, Gemini, …)

use super::*;

pub(super) fn provider_key(id: &str) -> String { format!("provider_{id}") }

pub fn set_provider_key(id: &str, key: &str) -> bool { set(&provider_key(id), key) }

pub fn get_provider_key(id: &str) -> Option<String> { get(&provider_key(id)) }

pub fn has_provider_key(id: &str) -> bool { get_provider_key(id).is_some() }

#[allow(dead_code)] // used when Settings adds a "remove key" control
pub fn clear_provider_key(id: &str) -> bool { clear(&provider_key(id)) }
