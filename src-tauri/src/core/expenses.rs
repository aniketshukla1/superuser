//! Expense Manager — receipts from your email, summaries that reconcile.
//!
//! The job: build this week's expense summary from the receipts in the finance inbox and the
//! attached policy, flagging missing categories and policy exceptions, and return the summary
//! and drafts without sending messages or changing reimbursements. Every exception cites the
//! policy, and the totals reconcile back to the source.
//!
//! Here that is three parts, each doing what it is good at:
//!
//! * **Intake.** A receipt that arrives by email is read by the model on this machine — never a
//!   cloud one; these are your purchases — into a few fields, which this code checks before
//!   anything is written. A model that misreads one costs a wrong line in a file you review.
//!   It is given no tools, so an email that tries to instruct it has nothing to instruct.
//! * **The ledger.** `<data>/expenses/ledger.csv`, one receipt a line, each carrying the email
//!   it came from. A spreadsheet can open it; a cell that would run as a formula can't.
//! * **The summary.** Totals are computed here, not by the model, so they reconcile: every
//!   total lists the receipts it adds up. The Expense Manager (read-only) reads the summary with
//!   the policy and flags exceptions, citing the rule each one breaks.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};

use crate::integrations::email::Inbound;

#[cfg(test)]
mod tests;

mod expense;
mod csv_small_strict;
mod intake;
mod summary;

pub use expense::*;
pub use csv_small_strict::*;
pub use intake::*;
pub use summary::*;
