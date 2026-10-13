//! Choosing tools by the skill that needs them.
//!
//! The argument, which is worth restating because it is the opposite of the obvious
//! one: **a retrieval prefilter fails categorically.** Rank tools by similarity to the
//! message and the model is handed a set that is usually right; the times it is wrong, the
//! work cannot be done at all and the model cannot tell why. So the unit here is the SKILL
//! — a thing written by a person, with a line saying when to use it and a declaration of
//! what it needs — and the failure mode is bounded: choosing a skill that turns out to be
//! unnecessary costs almost nothing, and every granted tool no skill claims is offered
//! anyway.
//!
//! Three ways this refuses to narrow, each of which offers everything:
//!
//! * fewer tools than the floor — narrowing a short list buys nothing;
//! * no skill declares any tools — there is nothing to narrow BY;
//! * the chooser could not answer — an unavailable model must not silently shrink the
//!   toolset, which is what "fails open" means here.
//!
//! Behind `SUPERUSER_SKILL_NARROWING=1` and measured against `find_tools` before anything
//! is adopted: our own ablation scored the relevance-ranked arm lowest, which is the
//! evidence that put this one behind a flag rather than in front of everybody.

use std::collections::BTreeSet;

use super::agent::ToolSpec;
use super::model_router::{self, Workload};
use super::settings::Settings;
use super::skills::Skill;

#[cfg(test)]
mod tests;

mod chosen;

pub use chosen::*;
