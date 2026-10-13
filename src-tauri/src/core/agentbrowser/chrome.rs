//! Finding a Chrome-family browser on this machine and starting one for an assistant.
//!
//! The default: CDP with the browser the user already has — Chrome,
//! Chromium, Brave or Edge — each assistant on its own `--user-data-dir`, so the user's
//! own profile is never opened and each assistant's sign-ins are its own.
//!
//! The flags are the common ones for an automated profile, with two choices of our own:
//! no `--password-store=basic` (a container workaround), but
//! `--use-mock-keychain` on macOS, because a separate profile still reaches for the
//! "Chrome Safe Storage" keychain item, and a system dialog popping up in front of the
//! user in the middle of an assistant's headless run is worse than a profile whose
//! cookies are protected by the folder's permissions instead.

use std::path::{Path, PathBuf};

#[cfg(test)]
mod tests;

mod launch;
mod clear_stale_locks;

pub use launch::*;
pub use clear_stale_locks::*;
