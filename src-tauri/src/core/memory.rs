//! Local-first Memory Tree: Obsidian-compatible markdown vault with YAML
//! frontmatter, plus a recency/relevance weight that decays over time and is
//! reinforced on use. Ported and hardened from the original JS prototype.
//!
//! Every memory also records **where its content came from** — see [`Origin`].
//! The vault is not just a cache of things worth knowing; it is an input to an
//! agent that can act, and content a stranger wrote has to be treated differently
//! from content the user wrote.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(test)]
mod tests;

// Minimal self-contained temp dir helper for tests (avoids an extra dep).
#[cfg(test)]
mod tempdir {
    use std::path::PathBuf;
    pub struct TempLike {
        pub path: PathBuf,
    }
    impl TempLike {
        pub fn new() -> Self {
            use std::sync::atomic::{AtomicU64, Ordering};
            static N: AtomicU64 = AtomicU64::new(0);
            let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
            // counter avoids collisions when tests start within the same nanosecond
            let p = std::env::temp_dir().join(format!("bba-test-{}-{}", nanos, N.fetch_add(1, Ordering::SeqCst)));
            std::fs::create_dir_all(&p).unwrap();
            Self { path: p }
        }
    }
    impl Drop for TempLike {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }
}

mod memory;
mod entities;
mod memory_engine;
mod same_content;

pub use memory::*;
pub use entities::*;
use same_content::*;
