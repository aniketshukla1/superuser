//! Where a cell's value came from.
//!
//! The tracker in the product this was modelled on has an "Evidence Source"
//! column that a human fills in by hand. That column is the whole difference
//! between a template and a document you can trust: without it, a spreadsheet
//! assembled by an agent is a pile of numbers you have to re-verify yourself,
//! which is most of the work you were trying to avoid.
//!
//! So a cell can carry its source — the file, email or page it was read from —
//! and the record lives beside the workbook rather than inside it. Beside,
//! because a column of URLs makes a spreadsheet worse to look at and worse to
//! print, and because the user editing the cell by hand should not have to
//! maintain a citation next to it.
//!
//! The claim this supports is narrow and worth stating exactly: it records what
//! the agent SAID it used when it wrote a cell. It is an audit trail, not proof —
//! but an audit trail you can follow back to a receipt is the thing no template
//! generator can offer.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;

mod describe;

pub use describe::*;
