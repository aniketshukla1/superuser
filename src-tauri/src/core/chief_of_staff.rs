//! Chief of Staff — what came in since yesterday, mapped to your priorities.
//!
//! The job: review activity since yesterday across your approved channels, inbox, calendar and
//! meeting notes, and return only the items that map to the priorities you wrote down. Each item
//! carries its source, why it matters, the proposed next step, and whether you owe a decision.
//! It never sends messages or changes meetings.
//!
//! Here the activity is what your connected apps already bring into memory — mail, calendars,
//! GitHub, Notion, Linear and the rest — plus the notes you keep yourself. `activity_since`
//! (read-only) gathers it in code, numbered A1, A2…, beside your priorities numbered PR1, PR2…;
//! the Chief of Staff (read-only) keeps only what maps to a priority and says, for each, where it
//! came from, why it matters, the next step, and whether you owe a decision. The code decides
//! what counts as recent; the model decides what matters. It sends nothing and changes no
//! meeting — it has nothing that could.

use chrono::{DateTime, Datelike, Duration, Local, TimeZone};
use serde_json::Value;

use crate::core::memory::{Memory, Origin};

#[cfg(test)]
mod tests;

mod digest;

pub use digest::*;
