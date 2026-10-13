//! Writing Word documents and slide decks.
//!
//! Reading these has worked for a while (`files::extract`); writing them has not,
//! so an assistant asked for "a one-page brief" or "three slides" could only ever
//! answer with markdown and an apology.
//!
//! Word goes through `docx-rs`. PowerPoint is built here, part by part, because no
//! Rust crate writes one — a .pptx is a zip of OOXML, and the reader in `files.rs`
//! already proves which parts matter. The deck this produces is deliberately plain:
//! a title and bullets per slide, no theme, no master beyond the minimum. A plain
//! deck that opens is worth more than a designed one that PowerPoint refuses.

use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;

mod content_slide;
mod write_pptx;

pub use content_slide::*;
pub use write_pptx::*;
