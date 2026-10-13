//! Local file reader. Reads a file off disk and turns it into something an LLM
//! can use — extracted text for documents/code, or base64 for images (which the
//! caller routes to a vision model). Everything happens on-device; nothing is
//! uploaded by this module.

use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::Serialize;
use std::path::Path;

#[cfg(test)]
mod document_reading_tests {
    use super::*;

    /// A document must come back as its words, not its bytes.
    ///
    /// Ignored by default because it needs real files to read; the fixtures are
    /// made by the caller. Run with:
    ///   SU_PDF=… SU_DOCX=… cargo test --no-default-features -- --ignored reads_a_real
    #[test]
    #[ignore]
    fn reads_a_real_document_as_text() {
        if let Ok(pdf) = std::env::var("SU_PDF") {
            let e = extract(&pdf).expect("pdf should extract");
            let text = e.text.unwrap_or_default();
            assert!(text.contains("Notion"), "pdf text missing: {text:?}");
            assert!(!text.contains("%PDF"), "raw bytes leaked into the text");
        }
        if let Ok(docx) = std::env::var("SU_DOCX") {
            let e = extract(&docx).expect("docx should extract");
            let text = e.text.unwrap_or_default();
            assert!(text.contains("renewal"), "docx text missing: {text:?}");
            assert!(!text.contains("<w:"), "raw OOXML leaked into the text");
        }
    }
}

#[cfg(test)]
mod tests;

mod extract;

pub use extract::*;
