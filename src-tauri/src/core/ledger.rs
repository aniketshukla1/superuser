//! The memory ledger — what changed in the vault, and when.
//!
//! The vault answers "what do I know". It cannot answer "what's new", and that is
//! the question every useful background behaviour actually asks. A morning brief
//! that re-reads the whole inbox says the same things every day; an automation
//! that polls a source has no way to tell a message it already handled from one it
//! hasn't. Both need a delta, and a delta needs a record of the past.
//!
//! Rather than invent a snapshot format, this maps onto git, because the vault is
//! already a folder of markdown files:
//!
//! | ledger idea | git |
//! |---|---|
//! | snapshot | a commit |
//! | what a source contributed | the files named `<source>-*.md` |
//! | "since I last looked" | a ref per source, `refs/superuser/read/<source>` |
//! | the delta | `git diff --name-status <marker>..HEAD -- <pathspec>` |
//!
//! Two properties follow for free and are the reason to do it this way. It costs
//! **no model calls** — the history is built from files already written by sync.
//! And the user gets a real, inspectable record of their own notes: `git log` in
//! the vault works, because it is an ordinary repository.
//!
//! # Read markers
//!
//! [`Ledger::changes_since_read`] returns everything since the marker and then
//! moves the marker to HEAD. Poll a source twice and the second call is empty —
//! which is what stops an assistant from reporting the same three emails every
//! twenty minutes. `commit: false` previews without acknowledging, for a caller
//! that wants to look without consuming.
//!
//! # Failure posture
//!
//! Everything here is best-effort and must never break a sync. No `git` on the
//! machine, an unwritable vault, a repository in a state we didn't expect: the
//! ledger reports that it is unavailable and the rest of the app carries on. A
//! missing history is a lost feature; a failed sync is lost data.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;

#[cfg(test)]
mod tests;

mod changes;
mod ledger;

pub use changes::*;
