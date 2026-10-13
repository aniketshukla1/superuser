//! The browser, for real: a headless Chrome on a throwaway profile, a site this test
//! serves itself on loopback, and every rule checked against what the page actually did.
//!
//! Ignored by default — it needs a Chrome-family browser installed — and run with:
//!
//!     cargo test --lib agentbrowser::live_tests -- --ignored --nocapture
//!
//! Nothing here touches the network beyond loopback, and nothing touches the user's own
//! Chrome profile: the profile lives in a temp directory that is deleted at the end.

use std::sync::atomic::Ordering;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::computer;

mod real_browser_fills;
mod real_browser_reports;

use real_browser_fills::*;
