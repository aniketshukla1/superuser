//! One assistant's browser — its profile, its pages, and everything done to them.
//!
//! Every browser action, in one place so there is one lock and one rule per action:
//!
//! * **One driver.** `Control` refuses every acting call while the user holds the wheel.
//! * **Refs belong to a snapshot.** The generation moves on every navigation, every
//!   in-page route change and every change of page, and an action naming an older
//!   snapshot is refused with "take a new snapshot".
//! * **Typed values are never echoed.** A result says how many characters went in.
//!   A password field is refused outright and pointed at `request_secret`, whose value
//!   travels from the user's keyboard to the page and nowhere else.
//! * **Nothing leaves except through the proxy.** See `proxy`.
//!
//! The browser starts on first use and stops after 30 idle minutes, or when a ninth
//! assistant needs one. Stopping keeps the sign-ins; `reset` is what signs out.

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use serde::Serialize;
use serde_json::{json, Value};
use tokio::sync::broadcast;

use super::cdp::{Cdp, Event};
use super::control::{Control, ControlView, Holder};
use super::snapshot::{self, Element};
use super::{chrome, keys, proxy};

#[cfg(test)]
mod tests;

mod shared;
mod where_things_live;
mod registry;

pub use shared::*;
pub use where_things_live::*;
pub use registry::*;
