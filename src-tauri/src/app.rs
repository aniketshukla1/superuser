//! The application itself: its state, startup, the background schedulers and the
//! local API. `lib.rs` only declares the crate's modules and re-exports this one.

use crate::{commands, core, integrations, mcp, tray, voice};
use crate::core::memory::MemoryEngine;
use crate::core::settings::Settings;
use crate::integrations::Registry;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

#[cfg(test)]
mod migration_tests;

#[cfg(test)]
mod tests;

mod migrate_legacy_data;
mod run;
mod start_scheduler;
mod start;
mod start_evolution_scheduler;
mod start_profile_scheduler;
mod start_app;
mod answer_telegram;
mod answer_email;
mod start_local_api;

pub use migrate_legacy_data::*;
pub use run::*;
use start_scheduler::*;
use start::*;
use start_evolution_scheduler::*;
use start_profile_scheduler::*;
use start_app::*;
pub use answer_telegram::*;
pub use answer_email::*;
use start_local_api::*;
