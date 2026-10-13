//! Context-window management for the assistant agent loop.
//!
//! Two pressures, two mechanisms:
//!
//! * **Across turns** — a session's history grows forever. `trim_turns` keeps the
//!   most recent turns that fit the budget and hands back the dropped ones so the
//!   caller can summarize them into one paragraph instead of dropping them silently.
//! * **Within a turn** — up to `max_steps` tool calls accumulate, and one
//!   `run_shell` or `read_file` can be megabytes. `truncate_tool_results` rewrites
//!   over-long results from *earlier* steps to head+tail with a notice, leaving the
//!   step the model is currently reasoning about intact.
//!
//! Truncating is preferred over dropping messages inside a turn: both APIs require
//! every `tool_use` to be answered by its `tool_result`, so removing one half of a
//! pair is a protocol error. Shrinking content keeps the pairing and still frees
//! the space.
//!
//! Both message shapes are handled — OpenAI-compatible (`{"role":"tool"}`) and
//! Claude native (a `user` message whose content is a list of `tool_result` blocks).

use serde_json::Value;

use crate::core::settings::Settings;

#[cfg(test)]
mod tests;

mod middle_out_recorded;
mod truncate_tool_results;
mod trim_turns;

pub use middle_out_recorded::*;
pub use truncate_tool_results::*;
pub use trim_turns::*;
