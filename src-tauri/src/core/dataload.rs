//! The numbers a chart or a table draws, read from the file that holds them — Phase 5.
//!
//! Until now a chart carried its rows inline: the model read a file, retyped the figures
//! into the spec, and the picture was only as good as the retyping. That is the step where
//! a chart stops matching the file it says it came from, and nothing downstream can tell —
//! the spec is internally consistent, the source line is right, and one number is wrong.
//!
//! So a chart or a table may name its source and leave the rows out, and the app reads
//! them. What is drawn is then the file, and "where did this come from" has the same
//! answer as "what does it say".
//!
//! A CSV keeps its own column order. A JSON row's keys come back in alphabetical order —
//! `serde_json` sorts them, and turning that off is a change to every JSON value this app
//! holds — so a chart over JSON should name the fields it wants rather than relying on
//! which came first.
//!
//! Deliberately small: CSV, TSV and JSON arrays of objects. A spreadsheet goes through
//! `sheet_read` first — it has sheets, ranges and formulas, and guessing which sheet was
//! meant is how the wrong numbers get drawn confidently.

use std::path::Path;

use serde_json::{Map, Value};

#[cfg(test)]
mod tests;

mod read;

pub use read::*;
