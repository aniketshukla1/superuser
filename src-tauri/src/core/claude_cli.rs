//! Handing a turn to the signed-in Claude Code CLI.
//!
//! The second provider that does not serve our tool loop, and for the same reason as
//! `codex_cli`: a Claude plan cannot be reached honestly from a third-party HTTP client.
//! Anthropic does not let other products offer claude.ai sign-in, and a subscription's
//! OAuth token lifted out of the CLI and replayed from here would be exactly that. What
//! is left is the CLI itself — Anthropic's own client, signed in by the user, running
//! its own agent — with this app handing it the turn and reading back what it did. We
//! never see a token; `claude` keeps it.
//!
//! Whether a turn delegated this way counts against the plan or is billed as extra usage
//! is Anthropic's call, not ours, and the provider's setup text says so rather than
//! promising either.
//!
//! The tools are read-only and not configurable from here, for the reason the Codex
//! sandbox is: the prompt is assembled partly from conversation content, and a turn
//! typed into a chat box should not be able to reach `Bash` or `Edit` through an agent
//! whose approvals nobody is present to answer. Editing work goes through our own tools,
//! which have their own approval path.

use std::path::{Path, PathBuf};
use std::process::Stdio;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;

#[cfg(test)]
mod tests;

mod argv;
mod run;

pub use argv::*;
pub use run::*;
