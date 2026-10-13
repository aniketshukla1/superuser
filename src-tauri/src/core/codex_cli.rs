//! Handing a turn to the signed-in Codex CLI.
//!
//! This is the one provider that does not serve our tool loop. `codex exec` runs
//! Codex's own agent — its tools, its sandbox, its steps — and gives back the
//! answer it finished with. So a turn routed here is a turn our loop does not run:
//! no MCP tools, no skills, no memory writes from tool results.
//!
//! That tradeoff buys the thing nothing else here can offer honestly. A ChatGPT
//! plan cannot be reached from a third-party HTTP client; the only client OpenAI
//! authorises for it is this CLI, signed in by the user. Shelling out to it means
//! the credential stays in the CLI's keeping, we never see a token, and OpenAI is
//! told who is actually calling. The alternative — lifting Codex's OAuth client id
//! and presenting ourselves as Codex — is what this module exists to avoid.
//!
//! The sandbox is `read-only` and not configurable from here on purpose. Codex is
//! being handed a prompt assembled partly from conversation content, and the CLI's
//! own flags reach as far as `danger-full-access`; a turn the user typed into a
//! chat box should not be able to reach that. Editing work goes through our own
//! tools, which have their own approval path.

use std::path::Path;
use std::process::Stdio;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;

#[cfg(test)]
mod tests;

mod run;
mod flatten_with;

pub use run::*;
pub use flatten_with::*;
