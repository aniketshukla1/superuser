//! Editing the text of a .pptx in place, shape by shape.
//!
//! The same argument as `docedit`, for the same reason. `docwrite::write_pptx` builds
//! a deck from titles and bullets, so "editing" a real presentation by regenerating it
//! would throw away its master, its theme, its images and every piece of layout the
//! designer put there — a deck rendered by `pptx-preview` in the panel looks nothing
//! like what would come back.
//!
//! So this changes the words in the shapes it is asked about and leaves everything
//! else exactly as it found it. A shape nobody touched is byte-identical afterwards,
//! and every part of the archive other than the one slide is copied across untouched.
//!
//! What an edit costs: a shape whose text is split across runs with different
//! formatting ("the **important** bit") collapses to the first run's formatting,
//! because the replacement text has no way to say where the bold used to start.
//! Untouched shapes keep theirs.

use std::path::Path;

#[cfg(test)]
mod tests;

mod shape_spans;
mod rebuilt;

pub use shape_spans::*;
use rebuilt::*;
