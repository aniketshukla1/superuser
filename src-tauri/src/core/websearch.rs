//! Web search + page fetch for the research agent. Uses DuckDuckGo's HTML
//! endpoint (no API key, no third-party broker — privacy-aligned), and strips
//! fetched pages to text. Network egress only happens when the caller allows it
//! (the chat command gates this behind the "local-only" switch being OFF).

use serde::Serialize;

#[cfg(test)]
mod tests;

mod search_result;
mod pure_helpers;
mod network;

pub use search_result::*;
pub use pure_helpers::*;
pub use network::*;
