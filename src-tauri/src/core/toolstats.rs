//! What each tool actually does, counted.
//!
//! TOOLS-RESEARCH-2026 §11: `tool_status` classifies one tool result and nothing
//! aggregates it, so every question worth asking about a 53-tool catalogue is
//! unanswerable. Which tools are never called? Which fail most, and on which model?
//! Is the resident set — the fifteen schemas paid for on *every* request — the right
//! fifteen? That list was chosen by measuring token cost, not use.
//!
//! This is the fold that answers them. It is deliberately a pure tally over
//! `(name, failed)` pairs with no I/O of its own, because the two things worth
//! counting live in different places and must produce the same table:
//!
//! * the scoreboard's `TurnRecord.calls` — tools under tasks we control, and
//! * `run.jsonl` — tools under whatever the user actually did.
//!
//! Keeping them in one function is the point. A per-tool table built two ways would
//! disagree about what "failed" means within a week.
//!
//! # Reading the output, and the trap in it
//!
//! **Never-called is not dead.** A tool can go uncalled because nobody needs it, or
//! because no task exercises it, or because its capability was not available in the
//! sessions counted (`files` needs a workspace; `shell` needs an executor). Only the
//! first is a reason to delete anything. The report separates the catalogue from what
//! ran precisely so the difference stays visible, and the scoreboard column is the
//! weaker evidence of the two: it says what nineteen fixed tasks touch, which is a
//! statement about the tasks.

use std::collections::BTreeMap;

#[cfg(test)]
mod tests;

/// What this machine's own sessions actually reached for.
///
/// Ignored by default: it reads a real data directory, which CI does not have. Run it
/// deliberately —
///
/// ```sh
/// cargo test --lib real_usage -- --ignored --nocapture
/// SUPERUSER_DATA_DIR=/somewhere/else cargo test --lib real_usage -- --ignored --nocapture
/// ```
///
/// Reads every `sessions/*/run.jsonl`. Note the sample's shape before reading
/// anything into it: `runlog::start` discards the previous turn, so each file holds
/// **one turn** — the last of that session. So this is "the final turn of N
/// conversations", not "everything that ever ran". It undercounts tools used early
/// and abandoned, and it is still the only record of real use that exists.
///
/// A tool proposed and never finished is counted in its own column, not as an error.
/// It was refused by a gate, parked for approval, or the turn ended under it — and
/// conflating that with a tool that returned "error:" libels the blocking ones:
/// `ask_user` waits for a person, and a person who never answered is the tool
/// working.
#[cfg(test)]
mod real_usage;

mod tally;

pub use tally::*;
