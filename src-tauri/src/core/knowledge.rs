//! The knowledge base — what is known about a *topic*, as opposed to when it was
//! learned.
//!
//! Three stores now, and the distinction between them is the whole point:
//!
//! | store | organized by | written by | answers |
//! |---|---|---|---|
//! | vault notes (`MemoryEngine`) | source | ingestion — a connector sync, a chat summary | "what came in from GitHub?" |
//! | journal (`journal::Journal`) | time | automatically, as context is trimmed and distilled | "what did we establish, and when?" |
//! | knowledge (here) | topic | the agent, deliberately | "what do I know about Postgres?" |
//!
//! A note is a record of something that arrived. A knowledge page is a thing the
//! agent chose to write down and will keep editing — it accumulates, gets
//! corrected, and links to its neighbours. That is why pages are upserted by slug
//! rather than appended: the second conversation about Postgres should improve the
//! Postgres page, not create a second one.
//!
//! Pages live under `vault/knowledge/`, deliberately below the vault root that
//! `MemoryEngine::list` scans non-recursively — otherwise every page would also
//! show up in the memory list as a duplicate.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

#[cfg(test)]
mod tests;

mod wikilinks;
mod knowledge;

pub use wikilinks::*;
