//! Agent tool execution — runs a catalog tool by name against a session context,
//! delegating to Superuser's existing handlers (`core::coder` for files/shell/
//! search/git, `memory` for recall, `websearch` for the web). This is the
//! executable half of the agent: the catalog (`core::agent`) defines the tool
//! schemas, this runs them. The model-driving loop calls `dispatch`.
//!
//! Unlike the Coder (which *proposes* diffs for the user to review), an assistant
//! executes directly, producing real files in its working folder — that's what a
//! "finished deliverable" means. Dangerous shell is still blocked by
//! `run_capture`'s denylist; interactive approval for consequential external
//! actions is a layer added with connectors.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Mutex;

use serde_json::Value;

use crate::core::agent::{normalize_todos, TodoItem};
use crate::core::coder;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod stall_tracking;

mod artifacts_since;
mod dispatch_local;
mod tool_ctx;
mod tool_ctx_agent;
mod prompt_user;
mod gate_run_shell;
mod dispatch_toolchain;
mod automations;
mod authoring_agent;
mod browser_approval;
mod browser_tool;
mod dispatch;

pub use artifacts_since::*;
pub use dispatch_local::*;
pub use tool_ctx::*;
use prompt_user::*;
use gate_run_shell::*;
use dispatch_toolchain::*;
use automations::*;
use authoring_agent::*;
pub(crate) use browser_approval::*;
use browser_tool::*;
pub use dispatch::*;
