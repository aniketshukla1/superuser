use super::*;

/// Is `i` the end of a sentence, or just a full stop doing another job?
///
/// The naive rule — "a period ends a sentence" — chops "v1.2", "e.g." and "Dr."
/// into fragments, and a voice that pauses inside a version number sounds like a
/// machine reading characters rather than a person reading words.
pub(super) fn ends_sentence(bytes: &[u8], i: usize) -> bool {
    let c = bytes[i] as char;
    if c == '!' || c == '?' {
        return true;
    }
    if c != '.' {
        return false;
    }
    // A digit on both sides: a decimal, not a stop.
    let prev_digit = i > 0 && (bytes[i - 1] as char).is_ascii_digit();
    let next_digit = i + 1 < bytes.len() && (bytes[i + 1] as char).is_ascii_digit();
    if prev_digit && next_digit {
        return false;
    }
    // A single letter before the dot is an initial or an abbreviation ("e.g.",
    // "J. Smith"), not the end of a thought.
    if i >= 2 {
        let prev = bytes[i - 1] as char;
        let before = bytes[i - 2] as char;
        if prev.is_ascii_alphabetic() && (before == '.' || before == ' ') {
            return false;
        }
    }
    true
}

/// Common abbreviations that end in a period without ending a sentence.
pub(super) const ABBREVIATIONS: &[&str] = &["mr.", "mrs.", "ms.", "dr.", "prof.", "sr.", "jr.", "st.", "vs.", "etc.", "e.g.", "i.e.", "approx.", "fig.", "no."];

pub(super) fn ends_with_abbreviation(s: &str) -> bool {
    let tail = s.trim_end().to_ascii_lowercase();
    let last = tail.rsplit(|c: char| c.is_whitespace()).next().unwrap_or("");
    ABBREVIATIONS.contains(&last)
}

/// The longest speakable piece at the front of `buf`, and how many bytes it used.
///
/// Returns `None` while the buffer holds only part of a sentence — unless `flush`
/// is set, which is the end of the stream saying "there is no more coming, speak
/// what you have". This is what lets speech start on the first sentence of an
/// answer instead of on the last.
pub fn next_chunk(buf: &str, flush: bool) -> Option<(String, usize)> {
    let bytes = buf.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if !buf.is_char_boundary(i) {
            i += 1;
            continue;
        }
        if ends_sentence(bytes, i) {
            let end = i + 1;
            let head = &buf[..end];
            // A trailing abbreviation is not a sentence end; keep reading.
            if !ends_with_abbreviation(head) {
                // Take any closing quote or bracket that belongs to this sentence.
                // Iterated as chars, not bytes: a curly quote is multi-byte, and
                // stepping over it one byte at a time would split it.
                let mut end = end;
                for ch in buf[end..].chars() {
                    if matches!(ch, '"' | '\'' | ')' | ']' | '”' | '’') {
                        end += ch.len_utf8();
                    } else {
                        break;
                    }
                }
                let chunk = buf[..end].trim().to_string();
                if !chunk.is_empty() {
                    return Some((chunk, end));
                }
            }
        }
        i += 1;
    }
    if flush {
        let chunk = buf.trim().to_string();
        if !chunk.is_empty() {
            return Some((chunk, buf.len()));
        }
    }
    None
}

/// Text that should never be spoken aloud, stripped before it reaches a voice.
///
/// A reply written for the eye contains things that are unbearable in the ear: a
/// forty-line code block, a table, a URL read character by character. Speaking the
/// prose and skipping the rest is what a person reading aloud would do.
pub fn speakable(text: &str) -> String {
    let mut out = String::new();
    let mut in_fence = false;
    for line in text.lines() {
        let t = line.trim_start();
        if t.starts_with("```") {
            in_fence = !in_fence;
            if !in_fence {
                out.push_str(" (code omitted) ");
            }
            continue;
        }
        if in_fence {
            continue;
        }
        // Table rows read as noise.
        if t.starts_with('|') && t.ends_with('|') {
            continue;
        }
        let mut line = line.to_string();
        // Markdown emphasis and heading marks are punctuation for the eye only.
        for m in ['*', '_', '#', '`'] {
            line = line.replace(m, "");
        }
        out.push_str(line.trim());
        out.push(' ');
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The currently-speaking process, so it can be stopped the moment it should be.
#[derive(Default)]
pub struct Speaker {
    pub(super) current: Mutex<Option<Child>>,
    /// Set when the whole answer should stop, not just the sentence in flight.
    /// Killing one chunk is useless on its own — the next one would start a
    /// moment later and the user would be talked over anyway.
    pub(super) cancel: std::sync::atomic::AtomicBool,
    /// The wav a neural engine produced for the sentence being played, deleted
    /// once it has been. A conversation would otherwise leave one file per
    /// sentence in the temp directory.
    pub(super) temp: Mutex<Option<PathBuf>>,
    /// Sentences waiting their turn.
    ///
    /// A reply arrives a token at a time, so its sentences are ready one after
    /// another. They queue rather than replace: a second call that cut off the
    /// first would mean only the last sentence of every answer was ever heard.
    pub(super) queue: Mutex<std::collections::VecDeque<String>>,
    /// Whether a worker is already draining the queue.
    pub(super) draining: std::sync::atomic::AtomicBool,
}
