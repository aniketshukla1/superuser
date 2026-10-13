//! Reading one email: its headers, and the text a person wrote in it.
//!
//! Just enough MIME for the email bot (`core::email_bot`): folded headers, RFC 2047 encoded
//! words, multipart bodies (nested), quoted-printable and base64, and UTF-8 or Latin-1 text,
//! with an HTML-only message reduced to its words. Written here because this build has no
//! mail parser to lean on, and kept small because what it feeds is a draft a person reads
//! before anything is sent — a message it reads imperfectly costs a worse draft, never a send.

use base64::{engine::general_purpose::STANDARD as B64, Engine as _};

#[cfg(test)]
mod tests;

mod qp_decode;
mod strip_html;

pub use qp_decode::*;
pub use strip_html::*;
