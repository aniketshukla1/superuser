//! User settings, persisted as JSON in the app data dir. Secrets (the cloud API
//! key) are NOT stored here — they live in the OS keychain via `secrets`.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(test)]
mod tests;

#[cfg(test)]
mod prefs_tests;

mod default_archive_after;
mod settings;
mod settings_settings;
mod settings_2;

use default_archive_after::*;
pub use settings::*;
pub use settings_settings::*;
