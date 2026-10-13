//! SuperContext — a read-only sweep of memory before the model sees turn 1, so a
//! new thread doesn't start cold.
//!
//! # The conflict this had to be designed around
//!
//! Step 1 made taint per-run and sticky: the moment a run reads content that came
//! from outside this machine, it loses its standing pre-approvals and every
//! outward action needs a human. That property is what closes the exfiltration
//! path — a poisoned synced note can no longer talk a loop holding `send_message`
//! into using it.
//!
//! A scout that pre-loads recalled memory into every first turn would taint every
//! first turn, because a synced vault is overwhelmingly external. (Measured on the
//! real one: 125 memories, 0 internal.) Taint that is always on is taint that
//! means nothing — standing rules would stop working everywhere, the gate would
//! get switched off, and the safety feature would reopen the hole by being
//! useless. Porting this feature as it exists elsewhere would have quietly
//! disabled the property Step 1 exists for.
//!
//! # What this does instead: two tiers
//!
//! ```text
//!   internal memory  ─► inlined, in full          no taint: these are your words
//!   external memory  ─► named, never inlined      no taint: no third-party text
//!                       (source · count · dates)   is in the brief at all
//! ```
//!
//! The second tier is built **only from data we produced**: the connector prefix
//! off the filename, a count, and a date range. No titles, no summaries, no
//! bodies — nothing an outside party wrote reaches the prompt, which is why the
//! brief can be free. The model is then told plainly what was withheld and what
//! reading it will cost, and `recall_memory` still taints exactly as it does
//! today. So the run starts knowing what exists, and the gate keeps its meaning.
//!
//! The honest cost: on a fully-synced vault the brief is mostly tier two, a list
//! of things the model has to ask for. That is the correct trade. A brief that
//! silently inlined a stranger's words would be more convenient and would make
//! every other guarantee in this app conditional.

use std::collections::BTreeMap;

use super::memory::{MemoryEngine, Origin};

#[cfg(test)]
mod tests;

/// A look at the brief this machine's own vault would produce, for a question
/// given on the command line. Ignored by default (it reads a real vault, which CI
/// doesn't have); run with:
///
/// ```sh
/// SUPERUSER_VAULT="$HOME/Library/Application Support/ai.superuser.app/vault" \
///   cargo test --lib real_vault -- --ignored --nocapture
/// ```
///
/// Kept because the interesting failure here is not a wrong assertion, it is a
/// brief that reads badly against a real vault — which no fixture will show you.
#[cfg(test)]
mod real_vault {
    use super::*;

    #[test]
    #[ignore]
    fn print_the_brief() {
        let Ok(vault) = std::env::var("SUPERUSER_VAULT") else {
            eprintln!("set SUPERUSER_VAULT to a vault path");
            return;
        };
        let m = MemoryEngine::new(std::path::PathBuf::from(vault));
        for q in ["what's been happening with the build", "superuser plan", "my profile"] {
            let brief = sweep(&m, q);
            println!("\n=== {q:?} ===");
            println!("{}", render(&brief));
        }
    }
}

mod sweep;
mod render;

pub use sweep::*;
pub use render::*;
