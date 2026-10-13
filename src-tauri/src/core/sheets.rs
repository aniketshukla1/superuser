//! Working inside a spreadsheet, cell by cell.
//!
//! The distinction that matters here is between *generating* a file and *editing*
//! one. A model that regenerates a workbook to change one number quietly discards
//! everything it did not think to reproduce — the user's own rows, their column
//! widths, the note they left in H4. So nothing here rewrites a workbook from a
//! model's idea of it: cells are set individually, the rest of the file is read
//! from disk and written back untouched.
//!
//! That is also what makes a document co-editable rather than merely produced. The
//! user can change a cell by hand between turns and the agent's next edit lands
//! beside it instead of on top of it.

use std::path::Path;

use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;

mod write_cells;
mod read_rows;

pub use write_cells::*;
pub use read_rows::*;
