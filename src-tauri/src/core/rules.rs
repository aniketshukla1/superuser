//! Rules — the small expression language a boundary is written in.
//!
//! A boundary a person wants is a sentence: "never write a file called .env", "never
//! send anything to an address outside my company". A table of columns can hold the
//! shapes somebody thought of in advance; an expression can hold the one this person
//! thought of. So a rule is written in CEL — the subset of it a boundary actually
//! needs.
//!
//! # Why hand-written, and not a CEL crate
//!
//! The `cel-interpreter` crate was the first choice and this the fallback. The fallback
//! won on the three things a boundary depends on:
//!
//! * **The semantics that matter are not CEL's defaults.** Here `contains()` and
//!   `matches()` ignore case, so "never click submit" also stops a button labelled
//!   SUBMIT. A stock interpreter's `contains` is case-sensitive, and the rule that
//!   looks right would be the one that lets the capitalised button through.
//! * **Errors are part of the product.** A broken deny rule refuses, so a person
//!   typing one has to be told exactly what is broken and where, in words, before it
//!   starts refusing their work. That message is written here, not translated from a
//!   parser generator.
//! * **Nothing to vet.** A few hundred lines with their own tests, no dependency whose
//!   upkeep and licence would have to be checked again every release.
//!
//! # What is supported
//!
//! Literals (`"text"`, `'text'`, `r"raw"`, integers, decimals, `true`, `false`,
//! `null`, `[lists]`), field access (`element.name`), `!`, `&&`, `||`, `==`, `!=`,
//! `<`, `<=`, `>`, `>=`, `in`, `+ - * / %`, `cond ? a : b`, and the functions
//! `contains`, `matches`, `startsWith`, `endsWith`, `size` and `lower` — each callable
//! as `contains(x, y)` or `x.contains(y)`.
//!
//! # What a result means
//!
//! Only `true` and `false` answer "does this rule apply". Anything else — a string,
//! a number, an error — is a broken rule, and it is the *caller* that decides what a
//! broken rule means, because the safe answer differs by list: a broken deny must
//! still deny, and a broken allow must not permit. See `core::boundaries`.

use std::collections::BTreeMap;

use serde_json::Value as Json;

#[cfg(test)]
mod tests;

mod val;
mod lexing;
mod parsing;
mod evaluation;

pub use val::*;
use lexing::*;
pub use parsing::*;
use evaluation::*;
