//! Installing a skill from somewhere else.
//!
//! A skill is instructions an agent will follow. Installing one is closer to
//! granting a permission than to downloading a file: whoever wrote it gets to
//! steer the assistant, inside whatever tools the session already has. So this
//! module is built around one rule — **fetching and installing are separate
//! steps**, and nothing reaches disk until a human has seen the actual text.
//!
//! `preview` fetches and parses; it never writes. `install` takes a preview the
//! user has approved and writes it. The agent has no tool for either: a page it
//! read could otherwise talk it into installing instructions it would then obey,
//! which is a prompt injection with a persistence mechanism attached.
//!
//! Deliberately narrow in this first cut: **SKILL.md text only**. Skills that ship
//! scripts are the more useful half of the ecosystem and the much harder consent
//! problem — "approve this shell script you have not read" is not a question a
//! dialog can ask honestly — so they are refused with an explanation rather than
//! silently installed without their scripts.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::core::skills::parse_skill;

#[cfg(test)]
mod tests;

mod resolve_source;

pub use resolve_source::*;
