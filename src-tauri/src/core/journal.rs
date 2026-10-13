//! The memory *timeline* — the layer that turns conversations into something that
//! compounds, sitting beside the vault rather than inside it.
//!
//! The vault (`MemoryEngine`) holds discrete notes: one file per thing, written
//! deliberately or synced from a connected app. It answers "what do I know about
//! X". It has no notion of *when* something was learned or of consolidating what
//! was learned into a smaller, truer summary over time.
//!
//! The journal adds three tiers:
//!
//! ```text
//! conversation  →  journal/daily/YYYY-MM-DD.md  →  journal/MEMORY.md
//!    (context)        (evidence, dated)             (curated, injected)
//!                          ↓
//!                 journal/recaps/YYYY-MM-DD.md
//!                     (interpretation)
//! ```
//!
//! * **Daily** files accumulate the **actual words** as they fall out of a
//!   conversation's context window — what was said, not what a model made of it.
//!   Cheap, append-only, created on first write so quiet days leave no empty files.
//! * **Recaps** hold the model's summary of the same turns, written at the same
//!   moment. They are a *view* of the evidence, never a replacement for it.
//!
//! * **MEMORY.md** is the distilled core: preferences, decisions and durable facts.
//!   It is injected into *every* prompt, so it has to stay small — Deep Dream
//!   rewrites it rather than letting it grow.
//! * **Dream diaries** record what each consolidation actually did, so a rewrite of
//!   the core memory is reviewable rather than something that silently happened.
//!
//! # Why evidence and not the summary
//!
//! Daily and recap used to be one file: the summary was written into the daily and
//! the nightly pass then distilled *that* into the core memory. Which makes the core
//! a summary of summaries — every pass compounding the previous pass's reading of the
//! conversation, with the original words kept nowhere. A wrong turn early becomes
//! unfalsifiable, because there is no longer anything to check it against.
//!
//! So the daily file holds the transcript excerpt, the recap sits beside it, and
//! consolidation reads the evidence. The recap stays because it is genuinely useful
//! to read back — just not as the thing everything downstream is built on.
//! (The same rule OpenHuman's archivist states outright: the LLM recap is never
//! what gets ingested.)
//!
//! Everything lives under `vault/journal/` — deliberately *not* the vault root,
//! which `MemoryEngine::list` scans non-recursively for `*.md`. Putting MEMORY.md
//! there would make it show up in the UI as a stray note called "MEMORY".

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

#[cfg(test)]
mod tests;

mod entry;
mod journal;
mod searchable_chunks;
mod deep_dream;

pub use entry::*;
pub use searchable_chunks::*;
pub use deep_dream::*;
