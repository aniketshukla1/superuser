//! Watching what happens on the far side of a compaction (HARNESS-RESEARCH-2026 §3).
//!
//! `context::trim_turns` drops whole older turns and `summarize_dropped` folds them
//! into a paragraph. That is compaction, and the 2026 ablation work says to
//! instrument it before scaling it — because it does not degrade an agent evenly,
//! it makes it erratic. The measured shape: summary-conditioned agents terminated
//! correctly 44.6% of the time against 77.2% for dumb FIFO truncation at the same
//! budget, and pass² fell faster than the mean, which is the signature of
//! unreliability rather than of uniform loss.
//!
//! Three failure modes were named, each with a signal countable without a judge.
//! Two of them are observable from inside a run and are what this module counts:
//!
//! * **Regressive exploration** — the agent re-fetches. Measured at +0.031 refetches
//!   per step immediately after a compaction. Here: a call whose name and arguments
//!   exactly match one that was answered in the turns just dropped. It asked for
//!   something it had already been told, and had taken away.
//! * **Blocked actions** — +0.108 per step in the paper. Here: a call that comes
//!   back refused.
//!
//! The third, execution-state mislocalization, needs to know what the task wanted
//! and so belongs to the scoreboard rather than to the loop.
//!
//! Deliberately only a counter. It does not change what is compacted, and reads
//! nothing a person would have to trust — a refetch is string equality on a call
//! that already happened. The point is to have the number before the next round
//! proposes compacting harder, so the proposal can be argued against something.
//!
//! The window is three steps because that is what the paper measured. After that
//! the agent has re-established where it is, and anything counted would be ordinary
//! work rather than recovery from the boundary.

use serde_json::{json, Value};

#[cfg(test)]
mod tests;

mod watch;

pub use watch::*;
