//! Scale tests — what the local-first stores do when the data stops being small.
//!
//! Everything here is `#[ignore]`d: these build thousands of files and measure
//! wall-clock time, which is the wrong thing to do on every `cargo test`. Run them
//! deliberately:
//!
//! ```sh
//! cargo test --lib scale -- --ignored --nocapture
//! ```
//!
//! # Why this exists
//!
//! Every store in this app is files on a disk the user owns, read on demand with
//! no index and no database. That is the right trade for a product whose whole
//! claim is that your data is yours — but it means the cost of a read is the size
//! of the store, and the failure mode is not an error, it is a lag that arrives
//! quietly a year in when nobody is watching for it.
//!
//! These print numbers rather than asserting tight ones. A threshold assertion on
//! a timing test fails on a loaded CI box and gets deleted; the assertions here are
//! deliberately loose enough to only catch a change of *complexity class*, and the
//! printed numbers are what a person reads.

#[cfg(test)]
mod tests;

/// Concurrency — the places where two things happen at once on purpose.
///
/// The app runs six background loops beside the user's own turn, and several of
/// them write to the same files. These are the shared-state hazards worth naming;
/// they are `#[ignore]`d with the rest because they spawn real threads and are
/// slow, not because they are optional.
#[cfg(test)]
mod concurrency;

/// Durability — what a reader sees while a writer is halfway through.
///
/// Every JSON store in the app is `fs::write` of the whole file: truncate, then
/// write. That is not atomic to a reader, and the readers here are not
/// hypothetical — the profile is read at the start of every conversation while a
/// background pass may be writing it, and the watch list is read by the API while
/// the tick loop writes it. A reader that catches the truncated moment gets a
/// parse failure, and every one of these stores falls back to *empty* on a parse
/// failure. Empty is indistinguishable from "you have no watches".
#[cfg(test)]
mod durability;


