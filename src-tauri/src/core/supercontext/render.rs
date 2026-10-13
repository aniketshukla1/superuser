use super::*;

/// Render the brief for the system prompt. Empty when there is nothing to say, so
/// a fresh install and an unrelated question both cost nothing.
///
/// The withheld half names what it is hiding *and what reading it costs*. A model
/// that is silently given less than exists will answer as though that were
/// everything; one that is told what it hasn't seen can go and ask.
pub fn render(brief: &Brief) -> String {
    if brief.is_empty() {
        return String::new();
    }
    let mut out = String::from("\n\n[CONTEXT — what this machine already holds on this subject. Gathered before the conversation started; not something the user just said.]");

    if !brief.standing.is_empty() {
        out.push_str("\n\nStanding instructions the user has written down. These apply to this answer:");
        for m in &brief.standing {
            out.push_str(&format!("\n\n### {}", m.title));
            if !m.body.trim().is_empty() {
                out.push_str(&format!("\n{}", m.body.trim()));
            }
        }
    }

    if !brief.inlined.is_empty() {
        out.push_str("\n\nFrom notes written here:");
        for m in &brief.inlined {
            out.push_str(&format!("\n\n### {}{}", m.title, if m.date.is_empty() { String::new() } else { format!(" ({})", m.date) }));
            if !m.summary.trim().is_empty() {
                out.push_str(&format!("\n{}", m.summary.trim()));
            }
            if !m.body.trim().is_empty() {
                out.push_str(&format!("\n{}", m.body.trim()));
            }
        }
    }

    if !brief.withheld.is_empty() {
        out.push_str("\n\nAlso here, but NOT read and NOT included above:");
        for w in &brief.withheld {
            // Dates go in brackets, and an unattributed group reads "with" rather
            // than "from". Both came out of watching a real model read this line:
            // "7 items from no recorded source, 2026-06-09 to 2026-07-25" came back
            // out of qwen as "7 items without a recorded source date range", having
            // run the source and the dates together into one phrase.
            let when = match (w.from.as_str(), w.to.as_str()) {
                ("", _) | (_, "") => String::new(),
                (f, t) if f == t => format!(" ({f})"),
                (f, t) => format!(" ({f} to {t})"),
            };
            let source = if w.source == UNATTRIBUTED {
                format!("with {UNATTRIBUTED}")
            } else {
                format!("from {}", w.source)
            };
            out.push_str(&format!("\n- {} item{} {source}{when}", w.count, if w.count == 1 { "" } else { "s" }));
        }
        out.push_str(
            "\n\nNone of these are confirmed as written on this machine, so their contents are \
             deliberately kept out of this prompt. Use `recall_memory` to read them when the \
             question needs them — doing so puts this run behind the approval gate, so anything \
             acting outward will then ask the user first. Don't guess at what they say.",
        );
    }
    out
}
