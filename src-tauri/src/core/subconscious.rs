//! The subconscious — the loop that *decides* rather than the ones that fire.
//!
//! The app already runs four background loops: sync, automations, the nightly
//! consolidation, self-evolution. Every one of them does fixed work on a clock.
//! None of them ever looks at what actually changed and asks whether it matters.
//! So an automation that watches your inbox either runs on every tick and burns a
//! model call on a quiet morning, or runs rarely and misses the thing you wanted
//! it for.
//!
//! A **watch** is the other shape: a standing question about one or more sources,
//! evaluated only against what has changed since the last look.
//!
//! ```text
//! tick ─► for each enabled watch
//!           ├─ peek at the ledger delta for its sources
//!           ├─ nothing changed?           ─► skip, no model call at all
//!           ├─ ask the LOCAL model:        skip │ act │ escalate
//!           ├─ act      ─► answer it locally, record the note
//!           ├─ escalate ─► park an ask in the Inbox for a real look
//!           └─ acknowledge the delta — but only if the evaluation worked
//! ```
//!
//! # Why the local model
//!
//! This is the loop that makes an Ollama-first design pay for itself. Deciding
//! "is any of this worth acting on" is cheap, frequent, and exactly what a small
//! local model is good at; the expensive path is reserved for the minority of
//! ticks that earn it. A cloud-only assistant cannot offer this at all — nobody
//! wants a background loop billing them every five minutes.
//!
//! # Two rules that keep it honest
//!
//! **A tick that fails changes nothing.** The delta is peeked, not consumed, until
//! the evaluation has actually succeeded. A model that was down must not cause the
//! items it never saw to be marked as seen.
//!
//! **Nothing consequential happens unattended.** `act` is answering a question
//! locally — reading, summarizing, writing a note. Anything that reaches outside
//! goes through the Inbox like every other unattended action, so the worst a
//! poisoned email can achieve is a card asking the user a question.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::core::ledger::{Changes, Ledger};
use crate::core::settings::Settings;

#[cfg(test)]
mod tests;

mod decision;
mod subconscious;
mod tick;

pub use decision::*;
use subconscious::*;
pub use tick::*;
