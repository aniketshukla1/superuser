use super::*;

/// Does this text read like an attempt to redirect the model rather than inform it?
///
/// Used on content that reaches the prompt without a human ever seeing it — most
/// importantly the **descriptions an MCP server gives its own tools**, which are
/// written by whoever runs that server and are injected into every turn's tool
/// schema. A description is supposed to say what a tool does; one that says what
/// the model should do instead is not describing anything.
///
/// Heuristic, and deliberately used only to *lower trust*, never to block: a false
/// positive costs a run its pre-approvals, which is recoverable, while blocking a
/// legitimate tool on a substring match would not be.
pub fn reads_as_injection(text: &str) -> bool {
    let normal = normalize_for_matching(text);
    OVERRIDE_PHRASES.iter().any(|p| normal.contains(p))
}

/// Fold the text down to the form the phrase list is written in.
///
/// A raw `contains` on lowercased text is defeated by a second space, a line
/// break, a zero-width joiner, or a `1` in place of an `i` — none of which change
/// what a model reads, all of which change the bytes. Since this check only
/// *lowers trust* rather than blocking, folding aggressively is the right trade:
/// the cost of a false positive is a run losing its pre-approvals.
pub(super) fn normalize_for_matching(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut pending_space = false;
    for c in text.chars() {
        // Zero-width and bidi controls carry no meaning to a reader but split
        // any substring they're dropped into.
        if matches!(c, '\u{200b}'..='\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2060}' | '\u{feff}') {
            continue;
        }
        if c.is_whitespace() {
            pending_space = !out.is_empty();
            continue;
        }
        if pending_space {
            out.push(' ');
            pending_space = false;
        }
        // Leetspeak stand-ins. Only applied to characters that are already
        // ambiguous in this position; ordinary prose is unaffected because these
        // folds turn digits into letters, not the reverse.
        let c = match c {
            '0' => 'o',
            '1' | '!' | '|' => 'i',
            '3' => 'e',
            '4' | '@' => 'a',
            '5' | '$' => 's',
            '7' => 't',
            other => other,
        };
        out.extend(c.to_lowercase());
    }
    out
}
