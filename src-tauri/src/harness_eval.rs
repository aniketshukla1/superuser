//! The scoreboard — does the harness actually work, on this model?
//!
//! HARNESS-PLAN H9: `adversarial.rs` is 730 lines of excellent hostile-input probing
//! and contains **nothing about the model loop**. There is no test that asks whether a
//! task completes, whether tool selection stays correct, or whether a prompt change
//! regressed anything. So "the harness is excellent on model X" has never been a claim
//! anyone could check.
//!
//! That gap is not academic any more. Three phases have now landed with exit criteria
//! that read *"the scoreboard improves"* and HARNESS-BASELINE has had to record, three
//! times, that there is no scoreboard:
//!
//! * **Phase 5** cut a small model's tool schema by 77%. Cheaper — but does it finish
//!   more tasks, or fewer?
//! * **Phase 6** deferred 38 of 53 tools behind a search. Does tool selection hold up?
//! * **Phase 4** gave the model its own transcript back. Does that convert into
//!   completed work, or only into saved tokens?
//!
//! None of those are answerable by reading the code, and all of them are answerable by
//! running twenty fixed tasks and counting.
//!
//! ## The shape
//!
//! A [`Task`] is a prompt, a fixture to put in the working folder, and a `check` over
//! the [`TurnRecord`] the turn produced. The checks are **pure functions over a
//! record** — no network, no model — so they are unit-tested here against synthetic
//! records the same way any other logic is. What needs a model is only the driver that
//! fills the records in, and that lives behind `#[ignore]` at the bottom.
//!
//! ## What is asserted, and what deliberately is not
//!
//! Every check is mechanical: a file's contents, a substring, the order of two tool
//! calls, a number. Nothing asks a model to judge another model's answer — a rubric
//! grader would make the scoreboard's own reliability the thing in question, and the
//! point of this file is to be the fixed thing everything else is measured against.
//!
//! The cost is coverage: "did it explain its reasoning well" is not here, because it
//! cannot be checked without judgement. That is the right trade for a regression gate.
//!
//! ## The brain group, and why it needs its own instance
//!
//! Every group above grades the loop. None of them grades the *brain*: for nineteen
//! tasks and three phases, no task in either split read the vault, the journal, the
//! knowledge pages or the profile, so "recall works" was the one claim here that
//! nobody could check (BRAIN-RESEARCH-2026 §3).
//!
//! The obstacle was never the checks — they are `said(...)` like all the rest. It is
//! that a brain does not fit in a working folder. The vault lives in the app's data
//! directory, so a task that wants to grade recall has to seed the store the
//! retriever actually reads, and seeding means **clearing first**, or the previous
//! task's memories answer this one's question.
//!
//! Pointing that at the default port would clear the user's own vault. So the brain
//! tasks run only against an instance that is provably disposable:
//!
//! ```sh
//! export SUPERUSER_DATA_DIR=/tmp/su-eval          # the app, and the driver, agree
//! export HARNESS_EVAL_PORT=8799                   # not the one `tauri dev` takes
//! cargo test --lib scoreboard -- --ignored --nocapture
//! ```
//!
//! The driver compares its own `SUPERUSER_DATA_DIR` against the directory the app
//! reports, after `canonicalize`, and refuses the brain tasks on anything less than
//! an exact match. A skipped task is a hole in the table; a wiped vault is someone's
//! notes.
//!
//! A scratch instance starts no connector sync and no Slack or Telegram watcher: it reads
//! the real `secrets.json`, so those would copy the person's accounts into the scratch vault
//! (and into the brain tasks' answers) or reply through their real bots. See
//! `core::instance::is_scratch`.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// The driver
// ---------------------------------------------------------------------------
//
// Everything above is pure and runs in the normal suite. This part needs a model, so
// it needs the app: it drives the real turn through the real local API, which is the
// only way a scoreboard measures the harness people actually use — personas, gates,
// MCP, the loop shape, all of it — rather than a stripped-down copy of it.
//
//     cargo test --lib harness_eval::driver -- --ignored --nocapture
//     HARNESS_EVAL_MODELS="llama3.2:latest,gemma4:12b" cargo test … -- --ignored --nocapture
//
// The app must be running. Each task gets its own session id, which fixes its working
// folder (`session_dir` is deterministic for a session with no record), so fixtures go
// in before the turn and the files come back after it.
#[cfg(test)]
mod driver;

mod turn_record;
mod task;
mod browser_fixture_sites;
mod assistants_handoff_task;
mod vault_fixtures;
mod routine_group_check;
mod authoring_group_checks;
mod handoff_group_checks;
mod browser_group_checks;

pub use turn_record::*;
pub use task::*;
use browser_fixture_sites::*;
use assistants_handoff_task::*;
pub use vault_fixtures::*;
use routine_group_check::*;
use authoring_group_checks::*;
use handoff_group_checks::*;
pub use browser_group_checks::*;
