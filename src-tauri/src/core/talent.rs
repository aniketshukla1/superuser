//! Talent Scout — candidates who meet the must-haves, none already in your ATS, and outreach
//! drafts nobody sends.
//!
//! The job: find candidates who meet a role's must-haves, leave out anyone already in the
//! applicant tracking system, give the evidence for each match, and draft outreach in the
//! person's own voice — contacting nobody.
//!
//! Here **the exclusion is the code's.** `candidates_check` matches each candidate against the
//! person's ATS export (every ATS exports a CSV; its columns are found by their headers) and
//! against everyone shortlisted here before — by email, by profile address, or, weakest and said
//! so, by name. `candidates_shortlist` keeps the final list in `<data>/talent/shortlist.csv` and
//! refuses anyone the export already has, so a model that skips the check still can't put an
//! applicant on the list. The ledger holds only what recognising someone takes. The Talent Scout
//! has no tool that sends anything: outreach stays a draft.

use std::path::{Path, PathBuf};

use serde_json::Value;

#[cfg(test)]
mod tests;

mod person;
mod export;
mod shortlist;
mod tools;

pub use person::*;
pub use export::*;
pub use shortlist::*;
pub use tools::*;

/// The assistant (`personas_builtin/talent.md`).
pub const ASSISTANT: &str = "talent";
