//! Work with apps via MCP: let the model act through connected app tools

mod mcp_commands;
mod mcp_agent_run;

pub(super) use mcp_commands::*;
pub use mcp_agent_run::*;
