//! MCP over stdio — a server we launch and speak to on its own stdin/stdout.
//!
//! The other transport in this module is Streamable HTTP, which reaches servers
//! someone else is hosting. Most of the MCP ecosystem is not hosted: it ships as
//! `npx @scope/some-server` or `uvx some-server`, which is why a registry that
//! could store a stdio config but never talk to one left most of the ecosystem
//! listed and unusable.
//!
//! The framing is newline-delimited JSON — one JSON-RPC message per line. (This is
//! NOT the `Content-Length` framing LSP uses; LSP and MCP differ here, and
//! mixing them up produces a server that hangs rather than one that errors.)
//!
//! Every exchange is bounded by a timeout. A tool server is somebody else's
//! program: it can hang, and a turn that waits forever on it is worse than one
//! that reports the server didn't answer.

use std::process::Stdio;
use std::time::Duration;

use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

#[cfg(test)]
mod tests;

mod stdio_session;

pub use stdio_session::*;
