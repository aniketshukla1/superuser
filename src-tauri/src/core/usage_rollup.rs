//! What you have actually spent, today and this week.
//!
//! This exists because the composer wanted a "plan usage" block and there is no such
//! thing here: Superuser talks to whatever provider you configured, and none of them
//! tell us about your subscription. Rather than draw a bar with an invented number
//! behind it, this reports the one usage figure the app genuinely owns — the tokens in
//! its own session history.
//!
//! The honesty problem is real and it is handled explicitly. Message usage is optional:
//! providers that report nothing, and every message written before usage was recorded,
//! carry `None`. `None` means UNKNOWN, not zero. So each window reports how many
//! assistant turns it could account for alongside the total, and the UI can say "of the
//! N turns today, M reported their cost" instead of implying the total is complete.

use chrono::{DateTime, Datelike, Duration, Local, TimeZone, Timelike};

use super::sessions::Session;

#[cfg(test)]
mod tests;

mod rollup_at;

pub use rollup_at::*;
