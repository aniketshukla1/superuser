//! Bug Reproduction — a bug report reproduced on staging, and nowhere else.
//!
//! The job: read a bug report and reproduce it on staging with a fresh test account, returning
//! exact steps, expected and actual behaviour, screenshots, browser and OS details, relevant
//! console or network notes, and a minimal test case where possible. It never touches
//! production customer data, and test credentials are handed over securely, not in chat.
//!
//! Here: the Bug Reproduction assistant works in its own browser, **on the staging address set
//! in Settings and nowhere else** — opening, reading or acting on any other site is refused
//! before the browser is touched, and a staging link that redirects elsewhere keeps that page
//! from it, so production data never reaches it. Clicks and typing still ask, per
//! site, like every assistant's. A test account's password goes into the page through
//! `browser_request_secret`, never the chat. `browser_screenshot` keeps the screen at the moment
//! it matters, `browser_console` gives what the page reported on its own (console errors,
//! uncaught exceptions, failed requests) with the browser and OS, and the report and a minimal
//! test case are files in its folder.

use std::sync::{Mutex, OnceLock};

#[cfg(test)]
mod tests;

mod staging_problem;

pub use staging_problem::*;
