//! The silence watchdog — a model that stops talking ends the turn with a sentence.
//!
//! There was no timeout on a model
//! call at all. A provider that accepted the request and then sent nothing held the
//! turn open forever, with a spinner that looked exactly like thinking. Stop worked,
//! but only if someone was there to press it — and an automation has nobody.
//!
//! # Silence, not duration
//!
//! The stall guard measures how long *nothing has arrived*, never how long the
//! turn has taken. A long answer that keeps streaming is healthy
//! however long it runs; a stream that has gone quiet for a minute is not, however
//! short. The clock restarts on every chunk, and it counts only time spent waiting on
//! the provider.
//!
//! # Per provider
//!
//! A local model is allowed far longer, because its silences are real work: loading
//! weights, reading a long prompt on a laptop, or composing a large tool call that
//! Ollama does not stream until it is complete. Measured here: `gemma4:12b` spent 108
//! seconds on one step. Cloud providers send a first token in seconds and Anthropic
//! sends keep-alive pings, so a minute of nothing from one is a fault.
//!
//! When it fires, the request is abandoned, the turn ends with a sentence that says
//! what happened and what to do, and a `turn.stalled` row is written — last, after
//! the sentence, so recovering is never waiting on recording.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::Duration;

#[cfg(test)]
mod tests;

mod within;

pub use within::*;
