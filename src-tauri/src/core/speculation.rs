//! Doing the obvious next read while the model is still deciding to ask for it
//! (HARNESS-RESEARCH-2026 §5).
//!
//! The 2026 latency work mines historical traces for pairs where the second call
//! is predictable from the first, and pre-executes on them during generation.
//! Measured: 48.5% lower end-to-end latency, 67% less tool wait, p99 down 61.9% —
//! and 602 potentially side-effecting speculative actions detected and refused,
//! with zero divergence in final output.
//!
//! Their hard problem is deciding what is safe to speculate on. Ours is already
//! decided, in code, in two places: `RiskClass::Read` and `commands::parallel_safe`.
//! The policy here is exactly those and nothing new — *speculate only what we
//! already run in parallel, never on a tainted run, never past a gate.* Anything
//! that would need a new judgement call is not speculated.
//!
//! One pattern to start, and it is not a guess: `grep_before_reading_everything`
//! is a task on our own board, which is to say grep-then-read is a sequence this
//! harness already believes in enough to grade. When a grep comes back, the top
//! hit's file is very likely the next read.
//!
//! Why this fits us better than it fits the people who published it: our slowest
//! models are local ones where a step is tens of seconds, so the window between
//! "grep answered" and "model asks for the file" is enormous. The prefetch has
//! nothing to race.
//!
//! Two properties make a wrong guess free rather than harmful:
//!
//! * **A miss costs nothing but the read.** The speculative result is only ever
//!   served for the exact path that was prefetched. A model that asks for
//!   something else gets a normal read and the guess is dropped.
//! * **It cannot see outside the workspace.** The path is resolved against the run
//!   root and rejected if it escapes, before anything is opened. A speculative
//!   read is still a read, and the confinement that applies to the model's own
//!   reads applies to the one we make on its behalf.

use std::path::{Path, PathBuf};

#[cfg(test)]
mod tests;

mod speculator;

pub use speculator::*;
