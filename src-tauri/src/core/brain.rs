//! `@brain` — asking the brain itself, in the middle of a conversation.
//!
//! The four memory stores each have a surface of their own under Brain, and each
//! answers a different question (`BrainView.tsx` spells out the split). That is
//! right for browsing and wrong for asking: a question like "what did we decide
//! about the Postgres migration, and who was in that conversation?" is answered by
//! the *knowledge* page for the decision, the *journal* entry for when it was made,
//! the *memories* for the people, and the *profile* for how to say it back. Someone
//! holding that question should not have to know which of four screens holds their
//! answer, nor visit them one at a time.
//!
//! So `@brain <question>` reads all four at once and answers from the union.
//!
//! Three things distinguish this from the ordinary chat turn that already carries
//! some memory:
//!
//! 1. **Everything is retrieved, not sampled.** A chat turn folds in a handful of
//!    memories to stay inside a budget it must share with the conversation. This is
//!    the whole budget, spent on retrieval, because retrieval is the entire task.
//! 2. **The census is exact.** "How many repos do I have?" cannot be answered by
//!    reading a sample of memories, however well chosen — the answer is a count over
//!    everything, so we compute it and hand it over as fact. (The same reasoning as
//!    `commands::memory_ask`, which this supersedes for chat.)
//! 3. **The sources are ours, not the model's.** The "Where this came from" section
//!    is assembled here from what was actually retrieved, so a citation cannot be
//!    invented — the failure mode that makes a confident memory assistant worse than
//!    no memory assistant at all.
//!
//! Interception happens in the session socket (`api_server::handle_session_socket`),
//! not in the composer, so `@brain` works when typed by hand, from the `/v1` API, and
//! from any future front end — and unlike `/context`, both the question and the answer
//! are kept in the transcript, because they are a real exchange about real content.

use std::collections::BTreeMap;
use std::path::Path;

use crate::core::embeddings::{self, JournalHit};
use crate::core::journal::{Journal, Tier};
use crate::core::knowledge::{Knowledge, Page};
use crate::core::memory::{Memory, MemoryEngine};
use crate::core::model_router::{self, ChatMessage};
use crate::core::settings::Settings;

#[cfg(test)]
mod tests;

mod budget;
mod matched_categories;
mod recall;
mod recall_brain;
mod system_prompt;

pub use budget::*;
pub use matched_categories::*;
pub use recall_brain::*;
pub use system_prompt::*;
