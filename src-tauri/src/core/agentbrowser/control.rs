//! Who has the wheel — the assistant or the user.
//!
//! One browser has at most one driver. When an assistant meets a sign-in it can ASK for help;
//! the user takes control, does the part only they can do, and hands it back. While the
//! user holds it, every acting call from the assistant is refused — refused, never queued,
//! because a queued click lands after the person has moved on, and "two drivers on one
//! page is how an assistant clicks Confirm on a form a person was still filling in".
//!
//! No browser in here, and the clock is passed in, so every rule is a plain test.

use serde::Serialize;

#[cfg(test)]
mod tests;

mod control;
mod control_control;

pub use control::*;
