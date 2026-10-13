//! Editing a .docx in place, paragraph by paragraph.
//!
//! The tempting implementation is to read the text out, let the user change it, and
//! write the document back with `docwrite::write_docx`. That is destructive: our
//! writer emits plain paragraphs, so a real Word file — headings, bold, tables,
//! images, styles — would come back flattened, and the user would have lost work the
//! app never told them was at risk.
//!
//! So this works the way `sheets::write_cells` does: it edits the paragraphs it is
//! asked about and leaves everything else exactly as it found it. A paragraph nobody
//! touched is byte-identical afterwards, and every part of the archive other than
//! `word/document.xml` is copied across untouched.
//!
//! The one thing an edit does cost: a paragraph whose text is split across runs with
//! different formatting ("the **important** bit") collapses to the first run's
//! formatting when it is edited, because the new text has no way to say where the
//! bold used to start. Untouched paragraphs keep theirs.

use std::io::{Read, Write};
use std::path::Path;

#[cfg(test)]
mod tests;

mod paragraph_spans;

pub use paragraph_spans::*;
