//! Automation — a scheduled task is its own persistent entity; each fire is a fresh
//! Run of the task's instructions (its own thread + artifacts). The cron
//! next-run computation lives with the scheduler loop (built on top of this).
#![allow(dead_code)] // model + store consumed by the scheduler loop (later increment)

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;

mod gen_id;
mod standing_scoped_approvals;
mod store_automation;

use gen_id::*;
pub use standing_scoped_approvals::*;
pub use store_automation::*;
