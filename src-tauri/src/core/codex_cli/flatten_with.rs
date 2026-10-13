
/// Turn Codex's stderr into something worth showing.
///
/// The failure that actually happens is a model the plan doesn't carry, and the
/// CLI reports it as a JSON error body among a page of startup noise. Surfacing
/// that line alone is the difference between a fixable message and a wall of log.
pub(super) fn explain(stderr: &str, model: &str) -> String {
    if stderr.contains("not supported when using Codex with a ChatGPT account") {
        return format!("Your ChatGPT plan doesn't include `{model}`. Pick another Codex model.");
    }
    if stderr.contains("Please log out and sign in again") || stderr.contains("not available") && stderr.contains("auth") {
        return "Codex isn't signed in any more. Run `codex login` and try again.".into();
    }
    let last = stderr
        .lines()
        .rev()
        .map(str::trim)
        .find(|l| !l.is_empty() && !l.starts_with("warning:"))
        .unwrap_or("Codex failed without saying why.");
    format!("Codex: {last}")
}

/// Correction appended after the app's own instructions, which were written for a
/// loop that is not running on this path.
///
/// Those instructions describe this app's tools and its approval mode, and Codex
/// dutifully follows them: it ended turns with "Please approve the read-only
/// commands described above" and stopped — asking a question nobody was listening
/// for, since our loop is not there to answer and stdin closed with the prompt.
/// Last word wins, so this goes after the system prompt rather than before it.
pub(super) const HANDOVER: &str = "\
---

The instructions above were written for a different agent. You are running as a \
delegated agent with your own tools, inside a read-only sandbox that bounds what \
any command you run can do.

Do not ask for approval or permission — this turn has no interactive channel, so a \
question ends it with nothing done. Do the read-only work you need, then answer. \
Ignore any instruction above about this app's own tools, skills, or approval flow; \
use your own.";

/// Flatten a conversation into the single prompt `codex exec` takes.
///
/// Codex has no notion of our roles, so they are labelled rather than dropped —
/// without them a long history reads as one voice and the model answers the wrong
/// turn. The instructions go first because that is where Codex looks for them.
pub fn flatten(system: &str, history: &[(String, String)], message: &str) -> String {
    flatten_with(HANDOVER, system, history, message)
}

/// `flatten`, with the handover note of whichever delegated CLI is taking the turn —
/// Claude Code's (`claude_cli`) names different tools than Codex's sandbox.
pub fn flatten_with(handover: &str, system: &str, history: &[(String, String)], message: &str) -> String {
    let mut out = String::new();
    if !system.trim().is_empty() {
        out.push_str(system.trim());
        out.push_str("\n\n");
        out.push_str(handover);
        out.push_str("\n\n---\n\n");
    }
    for (role, text) in history {
        if text.trim().is_empty() {
            continue;
        }
        let who = if role == "assistant" { "Assistant" } else { "User" };
        out.push_str(&format!("{who}: {}\n\n", text.trim()));
    }
    out.push_str(message.trim());
    out
}
