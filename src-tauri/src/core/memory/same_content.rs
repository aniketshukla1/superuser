use super::*;

pub(super) fn unquote(s: &str) -> String {
    s.trim_matches(|c| c == '"' || c == '\'').to_string()
}

pub(super) fn esc(s: &str) -> String {
    s.replace('"', "'")
}

/// Whether a write would put back what is already there — the same words under the same title
/// and category. What decides whether a rewrite keeps its date (`MemoryEngine::save`).
pub(super) fn same_content(m: &Memory, input: &MemoryInput) -> bool {
    m.title == input.title && m.category == input.category && m.summary == input.summary && m.body.trim() == input.body.trim()
}
