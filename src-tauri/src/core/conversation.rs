//! Turn-taking — the difference between dictation and a conversation.
//!
//! Dictation is a button: you hold it, you talk, it types. A conversation has no
//! button, and everything hard about it is deciding *when a turn ends*. Too eager
//! and you cut the user off mid-thought; too patient and every exchange has a dead
//! pause in it. Both failures read as "this thing isn't listening to me".
//!
//! Three rules do most of the work here:
//!
//! **Two thresholds, not one.** Speech starts at a higher level than it stops at.
//! A single threshold sitting near the room's noise floor flaps — start, stop,
//! start — several times a second, and each flap is a turn boundary the rest of
//! the system has to believe.
//!
//! **A turn ends on silence, not on volume.** People pause mid-sentence to think.
//! The tail is long enough to survive a breath and short enough not to feel dead.
//!
//! **A cough is not a turn.** Speech has to persist before it counts as started,
//! so a chair creak cannot interrupt the assistant mid-answer.

use serde::{Deserialize, Serialize};

#[cfg(test)]
mod calibration_tests;

#[cfg(test)]
mod echo_tests;

#[cfg(test)]
mod speaking_tests;

#[cfg(test)]
mod tests;

mod turn_detector;
mod unsaid;
mod voice_for_persona;

pub use turn_detector::*;
pub use unsaid::*;
pub use voice_for_persona::*;
