//! Model providers — a registry of the LLM backends an assistant can run on
//! (17 providers + a curated tool-calling list). Most cloud providers speak the OpenAI chat-completions API, so one
//! OpenAI-compatible path covers many; Anthropic and Ollama have their own; Gemini
//! is registered but not yet wired into the tool loop.
//!
//! A model id may be `provider:model` (e.g. `openai:gpt-5.5`) or bare
//! (`claude-opus-4-8`, `llama3.2`) — `resolve` maps either to a provider.

use serde::Serialize;

#[cfg(test)]
mod tests;

mod providers;
mod runs_locally;
mod list_info;

pub use providers::*;
pub use runs_locally::*;
pub use list_info::*;
