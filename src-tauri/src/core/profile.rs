//! The user's profile — **how to behave**, as opposed to what is known.
//!
//! This is the third member of a set. `journal.rs` holds *facts on a timeline*,
//! `knowledge.rs` holds *what is known about a topic*, and until now nothing held
//! *how this person wants to be worked with*. That was captured once at onboarding
//! in a form and then frozen forever: an assistant who is told "stop explaining
//! things I already know" on Tuesday behaves identically on Wednesday.
//!
//! ```text
//!   the form (profile.json) ─┐
//!                            ├─► observations ─► score ─► facets ─► PROFILE.md
//!   the user's own turns  ───┘    (decaying)              (Active)   + system block
//! ```
//!
//! # What an observation is
//!
//! A single piece of evidence that the user prefers something, stamped with when
//! it was seen and **how it was learned** (its cue family). Observations are never
//! rewritten; a facet is derived from all of them, so a belief that stops being
//! restated fades on its own rather than needing something to go and delete it.
//!
//! # Two rules that keep this safe
//!
//! **Only the user's own words become observations.** This is the sharp end. The
//! profile is folded into the system prompt of *every* turn, so anything that can
//! write a facet can write into every future system prompt. Step 1 (memory
//! provenance) established that synced third-party content is `External` and has
//! to be treated as data; letting it teach the profile would hand a poisoned
//! calendar invite a permanent slot above the user's own instructions. `observe`
//! therefore takes user-authored turns and nothing else, and every value is
//! flattened to a single capped line so it cannot forge structure around itself.
//!
//! **The form is a standing declaration.** Fields the user typed into the profile
//! form are re-derived as observations on every load rather than stored once, so
//! editing one takes effect immediately and leaves nothing stale behind. They never
//! decay and they never have to earn their way past the evidence threshold — the
//! user typed them, so they apply. They can still be *outweighed* by what someone
//! says later, which is the point of the whole module: a name you set at onboarding
//! should lose to "actually, call me Nik" without your having to go and edit it.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(test)]
mod tests;

mod class;
mod value_max;
mod onboarding_form;
mod learned_profile;
mod per_session_freeze;
mod learning_from_conversation;

pub use class::*;
use value_max::*;
pub use onboarding_form::*;
pub use learned_profile::*;
pub use per_session_freeze::*;
pub use learning_from_conversation::*;
