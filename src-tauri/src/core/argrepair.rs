//! A second chance at the arguments, in a shape the model cannot get wrong.
//!
//! Two failures reach the model as text today. `read_arguments` catches a tool call
//! whose arguments were not JSON at all (H3), and `loopshape::check_args` catches
//! parsed ones of the wrong shape. Both hand the reason back so the call can be
//! retried rather than mourned, and both cost a step to do it.
//!
//! On Ollama the step is avoidable. The server compiles a JSON schema to a state
//! machine and masks every token that would leave it, so a call constrained to a
//! tool's own `parameters` schema **cannot** come back with a missing field or a
//! string where an integer belongs. Measured here on 2026-09-07 against Ollama
//! 0.32.14, on a schema with an `integer` and a three-way `enum`:
//!
//! | | native tool call | schema-constrained |
//! |---|---|---|
//! | `llama3.2:latest` | `"occurrence": "2"` — a string — **3/3** | correct integer, 3/3 |
//! | `qwen2.5:7b-instruct` | dropped a required field 1/3 | 3/3 |
//! | `gemma4:12b` | correct | correct |
//!
//! `check_args` validates `type`, so llama3.2's `"2"` is refused before dispatch on
//! every attempt. That is a step spent, three times out of three, on a model whose
//! whole step budget is twelve.
//!
//! ## Why this is a repair and not the normal path
//!
//! Because `format` and `tools` do not compose. Sent together, Ollama answers 200 and
//! `tool_calls` is **null** — the constrained object arrives in `content` instead. So
//! a schema cannot be laid over tool calling; it can only replace it. Constraining
//! every step would mean giving up multi-tool steps, the model's own choice of tool,
//! and the parallel reads that choice enables. Constraining the calls that already
//! failed costs nothing anyone was using.
//!
//! It follows that this needs no capability flag and no per-model list. It fires when
//! a call has *already* failed, so a model that emits good arguments never reaches it
//! — `gemma4` pays exactly nothing — and a model that emits bad ones was going to
//! spend the step anyway. The only gate is the wire: Ollama speaks `format`, and the
//! OpenAI-compatible `response_format: {type: "json_schema"}` is the same idea in
//! different words but is not measured here, so it is not sent.
//!
//! ## `think` is not optional here
//!
//! [`body`] pins `think: false`, and the reason is the largest number in the
//! measurement. On `gemma4:12b`, same prompt, one sample each:
//!
//! | request | wall | thinking emitted |
//! |---|---|---|
//! | `tools`, `think:false` | 7.5s | — |
//! | `tools`, `think:true` | 20.2s | 403 chars |
//! | `format`, `think:false` | 8.6s | — |
//! | `format`, `think:true` | 322s | — |
//! | `think:true`, no tools, no format | 298s | 8,284 chars |
//! | `format` + `think:true` | 570s | 16,537 chars |
//!
//! The grammar is not what costs the time; generation length is, and what collapses
//! generation length is having a tool to call. With tools on the wire this model
//! thinks for 403 characters and acts; take them away and it deliberates for pages
//! about how one might edit a file. A repair request has no tools by construction, so
//! it walks into exactly that — and there is nothing for it to reason about anyway,
//! the model having already chosen the tool. Inheriting the turn's `think` flag would
//! turn a three-second fix into a five-minute stall.
//!
//! ## What it does not promise
//!
//! Shape, not sense. A grammar cannot know that `notes.txt` is not
//! `/home/user/notes.txt`, and one run in the measurement invented exactly that. The
//! placeholder and grounding gates in `core::gates` are what catch an invented value,
//! and this makes them matter more rather than less. [`accept`] therefore re-runs
//! `check_args` on what comes back instead of trusting the server's guarantee: the
//! guarantee is about the schema we sent, and a mismatch between that and the one we
//! validate against is exactly the kind of thing that should fail loudly.

use serde_json::{json, Value};

#[cfg(test)]
mod tests;

#[cfg(test)]
mod printed_call_tests;

mod attempt;
mod json_objects;

pub use attempt::*;
use json_objects::*;
