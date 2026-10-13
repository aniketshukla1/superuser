//! What a turn cost, in dollars.
//!
//! Phase 0 of HARNESS-PLAN exists because "unmeasured optimization is folklore" —
//! and the plan's largest line item (§2.7, prompt caching, a projected 60–80% cut)
//! cannot be claimed or disproved without this file. The projections there are
//! arithmetic. These are the rates that arithmetic is done against, in one place, so
//! a later phase can be checked rather than believed.
//!
//! Two distinctions this module refuses to blur:
//!
//! * **Free is not unknown.** A model running on this machine costs nothing, and a
//!   model we have no rate for costs *something we cannot name*. Both would render
//!   as `$0.00` if they shared a representation, and the second one is a lie. Local
//!   models get `Some(0.0)`; unrecognised remote ids get `None`, and the UI says so.
//! * **Cache reads and writes are not input.** They are billed at 0.1× and 1.25×.
//!   Folding them into the input rate would make Phase 3 look like it did nothing
//!   (writes overcharged, reads undercharged, the two roughly cancelling).

use crate::core::sessions::TokenUsage;

#[cfg(test)]
mod tests;

mod rates_for;

pub use rates_for::*;
