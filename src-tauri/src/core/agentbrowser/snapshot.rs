//! What on a page an assistant can act on, as a short list with refs.
//!
//! The interactive elements, read straight from Chrome's
//! accessibility tree (`Accessibility.getFullAXTree`). An ARIA role, not a tag name, is
//! what decides: a `div role=button` is a button, and Chrome has already worked out which
//! is which.
//!
//! Refs are minted per snapshot (`e1`, `e2`, …, in document order) and are only good for
//! the snapshot that minted them. The generation moves on every navigation and every
//! change of page, and an action carrying an older one is refused with "take a new
//! snapshot" — a clearer thing to tell a model than a click that lands nowhere.
//!
//! No browser in here: the input is the tree Chrome returns, so every rule is a test.

use serde::Serialize;
use serde_json::Value;

#[cfg(test)]
mod tests;

mod elements_from_ax;

pub use elements_from_ax::*;
