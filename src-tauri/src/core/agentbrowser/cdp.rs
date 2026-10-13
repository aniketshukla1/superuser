//! One Chrome DevTools Protocol connection.
//!
//! Chrome is started with `--remote-debugging-pipe`, so the protocol runs over two pipes
//! this process holds — one JSON message per write, each ended by a NUL byte — and there
//! is no debugging port on the machine for any other program to reach. (A port would let
//! anything running as the user drive the assistant's signed-in browser.)
//!
//! Hand-written rather than a CDP crate for the reason `core::rules` is: the protocol
//! needed is two dozen commands, `serde_json::Value` covers every one of them, and the
//! generated crates compile the whole protocol to get them.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{json, Value};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::{broadcast, mpsc, oneshot};

#[cfg(test)]
mod tests;

mod cdp;

pub use cdp::*;
