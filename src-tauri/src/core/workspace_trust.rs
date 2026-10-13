//! Project-declared command allowances, and the trust that switches them on.
//!
//! A repository can say "in this project, `cargo test` and `git status` are
//! routine" by shipping a `.superuser/config.json`. That is genuinely useful —
//! the allowances travel with the repo, so every person working on it gets the
//! same quiet, and reviewing them is a normal pull request.
//!
//! It is also, stated plainly, **a file in someone else's repository asking for
//! permission to run commands on your machine.** Cloning a project must never be
//! the act that grants it. So the file does nothing at all until the user trusts
//! that exact folder, and this module is the record of which folders they have.
//!
//! Two decisions worth keeping:
//!
//! - **Trust follows the path, not a snapshot of the config.** Trusting a folder
//!   accepts its future allowances too, because the alternative — re-prompting on
//!   every edit — trains people to click through the prompt, and a prompt everyone
//!   clicks through protects nobody. The honest framing is the one the settings
//!   card uses: you are trusting the *project*, and you can revoke it there.
//! - **A declared allowance is a candidate, never a grant.** Everything the file
//!   asks for still goes through [`cmdgate::auto_allowable`], so it can widen what
//!   runs silently only within what that already considers vouchable: no
//!   substitutions, no redirects, no `xargs`, no `find -exec`. A repo cannot
//!   declare its way past the floor, and `hard_deny` never consults this at all.
//!
//! The setting lives in `.superuser/config.json`, which needs no new dependency.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;

mod trust_store;

pub use trust_store::*;
