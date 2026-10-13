//! Processes the agent starts and keeps an eye on — a dev server, a REPL, a watcher.
//!
//! The half of a shell that `run_shell` cannot do. `run_shell` waits for a command to
//! exit and hands back its output, which is right for `ls` and wrong for anything
//! that is meant to keep running: `npm run dev` never exits, a Python REPL waits for
//! input, `cargo watch` prints forever. Asked to do any of those, the agent either
//! hangs until the 120-second timeout or is told the command must terminate — and
//! "start the dev server and check the page loads" is an ordinary thing to ask a
//! local coworker.
//!
//! Terminal-Bench's top scorers say the same from the other end: the bump came from
//! letting the agent drive long-running and interactive commands — periodically
//! reading the running process's output and writing bytes back to its pty
//! (TOOLS-RESEARCH-2026 §4).
//!
//! # Shape
//!
//! Four tools, each small: `process_start` returns a handle; `process_read` returns
//! whatever the process has printed *since the last read* and whether it is still
//! running; `process_send` writes a line to its input; `process_stop` ends it. The
//! agent polls rather than being pushed to, which keeps a long-running process from
//! flooding the context window with output nobody asked for.
//!
//! # Governance, which is not optional
//!
//! * `cmdgate::hard_deny` is the floor on the command **and on every line sent**. A
//!   process that is itself a shell would otherwise be a way around the gate — start
//!   `bash`, then send the command the gate would have refused.
//! * `process_start` and `process_send` are `RiskClass::Exec`, so they go through the
//!   same approval path as `run_shell`.
//! * Handles are scoped to the session that made them. One conversation cannot read
//!   or drive another's processes, however it learned the handle.
//! * At most [`MAX_PER_SESSION`] live at once, and all of them die with the app.
//!
//! Each gets its own PTY for the reason `devserver.rs` gives: the process becomes a
//! session leader, so stopping it takes the whole tree — a bare `sh -c "npm run dev"`
//! would leave node orphaned holding the port.

use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use std::collections::HashMap;
use std::io::Write;
use std::sync::{Arc, Mutex, OnceLock};

#[cfg(test)]
mod tests;

mod start;
mod list;

pub use start::*;
pub use list::*;
