//! Interactive terminal via a PTY (pseudo-terminal). A real login shell runs
//! attached to the pty; the frontend (xterm.js) renders output and forwards
//! keystrokes. Output bytes stream to the UI base64-encoded over the
//! `terminal-output` event; `terminal-exit` fires when the shell exits.
//!
//! The shell is SHARED: the agent's `run_shell` tool runs its commands in this
//! same pty rather than in a detached process, so you watch it type. That is the
//! whole point — "see the background tasks it runs" is not a log, it's the actual
//! terminal doing the work in front of you.
//!
//! Sharing means the session cannot hang off `AppState`: `run_shell` is dispatched
//! from `agent_exec::dispatch_local`, which is a plain sync function with no Tauri
//! handle. So the session lives in a process-global, and the Tauri commands and
//! the agent reach the same one.

use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use std::io::Write;
use std::sync::{Condvar, Mutex, OnceLock};
use std::time::{Duration, Instant};

#[cfg(test)]
mod tests;

mod run_captured;
mod strip_ansi;

pub use run_captured::*;
pub use strip_ansi::*;
