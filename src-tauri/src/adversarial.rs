//! Adversarial / hostile-input probes.
//!
//! Scratch harness written during a deep end-to-end test pass. These are
//! deliberately *failing-first* probes: each one asserts the behaviour we would
//! want, so a failure names a real gap. Not part of the shipped suite's contract.

#![cfg(test)]

use std::path::PathBuf;

// ---------------------------------------------------------------- webguard ---

mod ssrf;

// ------------------------------------------------------------------ egress ---

mod egress_spine;

// ------------------------------------------------------------------- coder ---

mod coder_sandbox;

// ------------------------------------------------------- agent write tools ---

mod agent_tools;

// ---------------------------------------------------------------- sessions ---

mod session_ids;

// ----------------------------------------------------------------- privacy ---

mod redaction;

// ------------------------------------------------------------------- store ---

mod durability;

// ------------------------------------------------------------------- files ---

mod file_extraction;

// ---------------------------------------------------------------- settings ---

mod settings_edges;

// --------------------------------------------------------- durability audit ---

mod store_audit;

mod scratch;

use scratch::*;
