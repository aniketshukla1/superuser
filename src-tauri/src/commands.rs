//! Tauri command surface — the typed IPC boundary the React frontend calls.
//! Keep in sync with `src/lib/api.ts`.

use crate::core::memory::{Citation, Memory, MemoryInput};
use crate::core::model_router::{self, ChatMessage};
use crate::core::secrets;
use crate::core::settings::Settings;
use crate::AppState;
use serde::Serialize;
use tauri::State;

#[cfg(test)]
mod cu_and_plan_tests;

#[cfg(test)]
mod subagent_call_tests;

#[cfg(test)]
mod mcp_gate_tests;

#[cfg(test)]
mod stream_tests;

#[cfg(test)]
mod mcp_bundle_tests;

#[cfg(test)]
mod wire_order;

mod knowledge_graph;
mod chat;
mod memory_ask;
mod how_my_today;
mod plugins;
mod chat_history;
mod projects_commands;
mod computer_use;
mod work_apps_via;
mod messaging;
mod interactive_terminal;
mod app_browser;
mod mcp_tool_bundles;
mod user_profile;
mod remote_mcp_connectors;
mod custom_plugins;
mod personas_commands;
mod skills_commands;
mod model_providers;
mod automations;
mod multi_account_connectors;
mod local_openai_compatible;
mod audit_log;
mod oauth_connectors;
mod inbox_commands;
mod agent_run_loop;

pub use knowledge_graph::*;
pub use chat::*;
pub use memory_ask::*;
pub use how_my_today::*;
pub use plugins::*;
pub use chat_history::*;
pub use projects_commands::*;
pub use computer_use::*;
pub use work_apps_via::*;
pub use messaging::*;
pub use interactive_terminal::*;
pub use app_browser::*;
pub use mcp_tool_bundles::*;
pub use user_profile::*;
pub use remote_mcp_connectors::*;
pub use custom_plugins::*;
pub use personas_commands::*;
pub use skills_commands::*;
pub use model_providers::*;
pub use automations::*;
pub use multi_account_connectors::*;
pub use local_openai_compatible::*;
pub use audit_log::*;
pub use oauth_connectors::*;
pub use inbox_commands::*;
pub use agent_run_loop::*;
