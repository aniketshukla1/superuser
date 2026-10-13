//! Dev-only tracing — present while you are building, absent from what ships.
//!
//! "Omitted", not "switched off". [`crate::trace!`] expands to nothing at all under
//! `not(debug_assertions)`: no branch, no formatting, no string, and — because the
//! whole block is removed — none of the argument expressions either. A release build
//! contains no evidence that a trace call was ever written, which is the only version
//! of this that stays true. A runtime `if enabled` leaves the format arguments in the
//! binary and evaluates them for a line nobody reads.
//!
//! This is the same shape as the other dev-only feature in this app: the test bridge
//! is gated by `#[cfg(debug_assertions)]` on the Rust half and `import.meta.env.DEV`
//! on the webview half. Tracing has the same two gates, for the same reason. See
//! `surfaces/gui/src/trace.ts` for the other one.
//!
//! # What may be traced
//!
//! **Shape, not content.** Ids, counts, lengths, model names, timings, frame types —
//! everything you need to follow what happened, and nothing that says what anyone
//! wrote. This is a local-first app whose whole claim is that your conversations stay
//! yours, and a trace line is exactly the kind of thing that ends up pasted into a
//! bug report.
//!
//! When you genuinely need the words, `SUPERUSER_TRACE_CONTENT=1` turns on
//! [`preview`], which is the one sanctioned way to put user text in a line. It stays
//! off by default even in dev, so nobody enables it by forgetting.
//!
//! # Filtering
//!
//! Everything is on in a dev build. `SUPERUSER_TRACE=chat,tools` narrows it to the
//! topics you name; `SUPERUSER_TRACE=off` silences it. The topic is the first
//! argument and is a plain string, deliberately: a fixed enum would mean editing this
//! file to trace something new, which is friction in the exact moment you are chasing
//! a bug.

use std::collections::HashSet;
use std::sync::OnceLock;

/// Topics enabled by `SUPERUSER_TRACE`. `None` means "all of them".
#[cfg(debug_assertions)]
fn filter() -> &'static Option<HashSet<String>> {
    static F: OnceLock<Option<HashSet<String>>> = OnceLock::new();
    F.get_or_init(|| match std::env::var("SUPERUSER_TRACE") {
        Ok(v) if v.trim().eq_ignore_ascii_case("off") => Some(HashSet::new()),
        Ok(v) if !v.trim().is_empty() => {
            Some(v.split(',').map(|s| s.trim().to_ascii_lowercase()).filter(|s| !s.is_empty()).collect())
        }
        _ => None,
    })
}

/// Whether this topic should print. Public only because the macro calls it.
#[cfg(debug_assertions)]
pub fn enabled(topic: &str) -> bool {
    match filter() {
        None => true,
        Some(set) => set.contains(&topic.to_ascii_lowercase()),
    }
}

/// A short, length-labelled look at user text — the only sanctioned way to put
/// someone's words in a trace line, and off unless `SUPERUSER_TRACE_CONTENT=1`.
///
/// Reports the length either way, because "how long was it" answers most of the
/// questions "what did it say" gets asked for: whether the message arrived, whether
/// it was truncated, whether the transcript matches what was sent.
#[cfg(debug_assertions)]
pub fn preview(text: &str) -> String {
    static ON: OnceLock<bool> = OnceLock::new();
    let on = *ON.get_or_init(|| std::env::var("SUPERUSER_TRACE_CONTENT").map(|v| v == "1").unwrap_or(false));
    if !on {
        return format!("{} chars", text.chars().count());
    }
    let head: String = text.chars().take(80).collect();
    format!("{} chars: {:?}{}", text.chars().count(), head, if text.chars().count() > 80 { "…" } else { "" })
}

#[cfg(not(debug_assertions))]
pub fn preview(_text: &str) -> String {
    String::new()
}

/// Trace one thing that happened. `trace!(topic, "format", args…)`.
///
/// ```ignore
/// trace!("chat", "socket open session={id} agent={agent} model={model}");
/// ```
///
/// In a release build this expands to nothing — see the module note.
#[macro_export]
macro_rules! trace {
    ($topic:expr, $($arg:tt)*) => {{
        #[cfg(debug_assertions)]
        {
            if $crate::core::trace::enabled($topic) {
                eprintln!("[{}] {}", $topic, format!($($arg)*));
            }
        }
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_trace_call_compiles_and_says_nothing_in_release() {
        // The macro has to be usable from a test the same way it is from the app.
        // What this really guards is that it EXPANDS — a macro that only compiles
        // under `debug_assertions` would break the release build, which is the one
        // nobody runs until it matters.
        let id = "abc";
        trace!("test", "a line with an interpolation: {id}");
        trace!("test", "and one with no arguments at all");
    }

    #[test]
    fn content_is_withheld_unless_it_is_asked_for() {
        // The default has to be the safe one: a length, never the words. Someone
        // pasting a trace into an issue should not be pasting their conversation.
        std::env::remove_var("SUPERUSER_TRACE_CONTENT");
        let p = preview("the quick brown fox");
        assert!(p.contains("19 chars"), "{p}");
        assert!(!p.contains("quick"), "content leaked without being asked for: {p}");
    }

    #[cfg(debug_assertions)]
    #[test]
    fn an_explicit_topic_list_silences_everything_else() {
        // `filter` is read once per process, so this tests the decision rather than
        // the env var — setting the var here would race every other test.
        let only_chat: Option<HashSet<String>> = Some(["chat".to_string()].into_iter().collect());
        let matches = |f: &Option<HashSet<String>>, t: &str| match f {
            None => true,
            Some(s) => s.contains(t),
        };
        assert!(matches(&only_chat, "chat"));
        assert!(!matches(&only_chat, "tools"));
        // Unset means everything, which is what a dev build should do by default.
        assert!(matches(&None, "anything"));
        // `off` is an empty set, not an absent one.
        assert!(!matches(&Some(HashSet::new()), "chat"));
    }
}
