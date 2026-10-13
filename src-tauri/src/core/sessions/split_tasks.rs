use super::*;

/// Split a request into independent tasks, so one message can fan out into a
/// sub-thread per task.
///
/// Deliberately structural rather than model-driven: a bulleted or numbered list
/// is an unambiguous signal the user wrote several tasks, and detecting it costs
/// nothing and never surprises them. Prose like "do X and then Y" is NOT split —
/// guessing there would fork conversations the user meant as one. Returns empty
/// unless there are at least two usable items.
pub fn split_tasks(text: &str) -> Vec<String> {
    let mut items = Vec::new();
    for raw in text.lines() {
        let line = raw.trim();
        // "- x" / "* x" / "• x"
        let stripped = line
            .strip_prefix("- ")
            .or_else(|| line.strip_prefix("* "))
            .or_else(|| line.strip_prefix("• "))
            .map(str::to_string)
            .or_else(|| {
                // "1. x" / "2) x"
                let mut c = line.chars();
                let digits: String = c.by_ref().take_while(|ch| ch.is_ascii_digit()).collect();
                if digits.is_empty() || digits.len() > 2 {
                    return None;
                }
                // take_while consumed the delimiter; rebuild from the original.
                let rest = line[digits.len()..].trim_start();
                rest.strip_prefix('.').or_else(|| rest.strip_prefix(')')).map(|r| r.trim().to_string())
            });
        if let Some(item) = stripped {
            let item = item.trim().to_string();
            // Guard against list-shaped noise ("- ok", separators).
            if item.len() >= 4 {
                items.push(item);
            }
        }
    }
    if items.len() >= 2 { items } else { Vec::new() }
}

/// A line that only introduces the real request ("Can you take care of these for me?").
pub(super) fn is_lead_in(line: &str) -> bool {
    let l = line.trim().trim_end_matches(':').to_lowercase();
    l.ends_with('?') && l.len() < 60
        || matches!(
            l.as_str(),
            "hi" | "hey" | "hello" | "please" | "here you go" | "do these" | "todo" | "tasks"
        )
}

pub(super) fn strip_bullet(line: &str) -> &str {
    let l = line.trim();
    for p in ["- ", "* ", "• "] {
        if let Some(r) = l.strip_prefix(p) {
            return r.trim();
        }
    }
    // "1. x" / "2) x"
    let digits: String = l.chars().take_while(|c| c.is_ascii_digit()).collect();
    if !digits.is_empty() && digits.len() <= 2 {
        let rest = l[digits.len()..].trim_start();
        if let Some(r) = rest.strip_prefix('.').or_else(|| rest.strip_prefix(')')) {
            return r.trim();
        }
    }
    l
}

/// Drop polite and plumbing prefixes so the title starts at the actual task.
pub(super) fn strip_prefixes(text: &str) -> String {
    const PREFIXES: [&str; 20] = [
        "can you please ", "can you ", "could you ", "please ", "i want you to ",
        "i need you to ", "i'd like you to ", "help me ", "use the ", "use ",
        "call the ", "call ", "go ahead and ", "let's ", "lets ", "run the ",
        "now ", "just ", "okay ", "ok ",
    ];
    let mut t = text.trim().to_string();
    // Repeat: "Can you please use the X tool to …" sheds several layers.
    for _ in 0..4 {
        let lower = t.to_lowercase();
        let Some(p) = PREFIXES.iter().find(|p| lower.starts_with(**p)) else { break };
        t = t[p.len()..].trim().to_string();
    }
    t
}

/// Filler that says when or how eagerly, not what.
///
/// Removed anywhere in the phrase rather than only at the front: "call ask_user
/// RIGHT NOW with three questions" reads as "ask_user with three questions", and
/// the words being dropped carry none of what the chat was about.
pub(super) fn strip_filler(text: &str) -> String {
    const FILLER: [&str; 8] = [
        " right now", " right away", " for me", " if you can", " if possible",
        " straight away", " immediately", " please",
    ];
    let mut t = text.to_string();
    for f in FILLER {
        loop {
            let Some(i) = t.to_lowercase().find(f) else { break };
            t.replace_range(i..i + f.len(), "");
        }
    }
    t.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The first `n` words, ending on a word.
///
/// A name in a list is scanned, not read. A truncated sentence — "Ask_user right
/// now with THREE ques…" — costs a full read to learn what a glance should have
/// told you, and a column of them reads as noise.
///
/// SIX, not five. Five looked right until it turned "Write me a brief on Q3" into
/// "Write me a brief on…", which drops the only word that says what the chat was
/// about. English puts its subject late often enough that the cut has to leave
/// room for it.
pub(super) fn first_words(text: &str, n: usize) -> (String, bool) {
    let words: Vec<&str> = text.split_whitespace().collect();
    let cut = words.len() > n;
    (words.into_iter().take(n).collect::<Vec<_>>().join(" "), cut)
}

pub struct SessionStore {
    pub(super) root: PathBuf,
}
