//! Small pieces of desktop behaviour the GUI has always asked for: launch at
//! login, keeping the machine awake, and whether an update is available.
//!
//! All three were invoked and none existed. `invoke` swallows the error and hands
//! back null, so the toggles read as OFF and the update banner read as "you are up
//! to date" — the failure mode that looks exactly like the feature working.

use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;

mod autostart_entry;

pub use autostart_entry::*;
