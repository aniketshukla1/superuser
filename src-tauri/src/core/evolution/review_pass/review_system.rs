use super::super::*;

/// One file the review wants to write, as parsed from its answer.
#[derive(Debug, Clone, PartialEq)]
pub struct Edit {
    /// Relative to one of the allowed roots, e.g. `skills/release/SKILL.md`.
    pub path: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    /// Nothing worth changing and nothing worth remembering.
    Silent,
    /// Nothing the user can see changed, but the wiki learned something.
    ///
    /// This is the outcome that did not exist before the wiki layer, and it is now
    /// the common one. A review that notices a real pattern rarely has a skill fix
    /// ready on the first sighting; before, that observation was discarded and the
    /// second sighting started from nothing. The user is not told — no work means no
    /// message, and a pattern is not work — but the next pass inherits it.
    Noted { patterns: Vec<String> },
    /// Files really changed. `summary` is the one-line notice for the user.
    Changed { summary: String, files: Vec<String>, snapshot: String },
    Failed(String),
}

pub(in crate::core::evolution) const SILENT_TOKEN: &str = "<silent/>";

pub(in crate::core::evolution) const REVIEW_SYSTEM: &str = r#"You review a conversation that has just gone idle, and decide whether anything
about how you work should durably change because of it.

# Default to doing NOTHING

Most conversations need no change. Only act on a CLEAR signal from the list below.
If there is none, reply with exactly `<silent/>` and nothing else. Staying silent is
the normal, correct outcome — not a failure.

Greetings, small talk, acknowledgements, and ordinary questions answered fine are
NOT signals. For those, output `<silent/>` immediately.

# Signals worth acting on

1. SKILL — a skill used in this conversation showed a STRUCTURAL problem: a missing
   step, a wrong or outdated detail, an instruction that produced the wrong result.
   Rewrite that skill file with a small, incremental fix so it cannot recur.
   Fix the SOURCE, never just record the symptom: if a skill's instructions are
   wrong, editing the skill is the only thing that prevents it happening again.
   Writing "skill X has wrong detail Y" into memory does not.

2. NEW SKILL — a clearly reusable, repeatable workflow emerged that no existing
   skill covers and the user will plainly want again. Only when it is genuinely
   reusable; never for a one-off task.

3. PATTERN — you noticed something about *how the work went* that is worth
   remembering, even though no skill fix follows from it yet: a wall the work hit,
   or an approach that plainly worked. Write it as a pattern.

   This is the one signal you may act on by itself. A pass that records a pattern
   and changes no file is a good pass, not a wasted one — patterns compound, and the
   third sighting of the same pattern is what earns a skill change. If the wiki
   below already names this pattern, reinforce it by reusing its EXACT id rather
   than inventing a second name for the same thing.

4. MEMORY — RARE, and a last resort. Conversation summaries and a nightly
   consolidation already capture memory without you. Only add something if it is a
   durable fact, preference or decision that would visibly change future replies AND
   is not already recorded. Keep it to ONE short bullet appended to the core memory.

# Do NOT capture (these poison future behaviour)

- Environment failures: missing binaries, unset credentials, "command not found".
  The user can fix these; they are not durable rules.
- Negative claims about tools or features ("tool X doesn't work"). These harden into
  refusals the assistant later cites against itself.
- One-off task narratives. Summarising what happened today is not a reusable skill.
- Transient errors that resolved on retry inside the conversation.
- Anything already present in the files you were shown.
- Anything listed below as already rejected. The user undid that change once;
  proposing it again is worse than doing nothing.

The list above applies to patterns as much as to skills. A pattern is a durable
claim about how the work goes — not a diary of today.

# How to answer

Nothing to change AND nothing to record:
<silent/>

Otherwise emit any combination of the blocks below.

To record a pattern — the lightest thing you can do, and usually the right one:

<pattern id="short-slug" title="One line naming it" kind="failure">
What happened, the evidence for it from this conversation, and what would have
avoided it. Write it so a reader who never saw this conversation can act on it.
</pattern>

`kind` is `failure` (a way the work went wrong) or `strategy` (a way it went right
that is worth repeating). Reuse an existing id to reinforce that pattern.

To change a file, emit its COMPLETE new content. Never abbreviate a file or write
"unchanged" — what you emit replaces the file entirely.

<edit path="skills/example/SKILL.md">
---
name: example
description: ...
---
full file content here
</edit>

If a pattern motivated an edit, name it — new ids and ones already in the wiki both
count. This is recorded beside the skill so the change can be explained later:

<because>short-slug, another-slug</because>
<note>One plain sentence, addressed to the user, saying what you learned and changed.</note>

Rules for paths: only `skills/<name>/SKILL.md` and `journal/MEMORY.md` may be
written. Any other path is rejected."#;

/// One pattern the review wants written to the wiki.
#[derive(Debug, Clone, PartialEq)]
pub struct PatternEdit {
    pub id: String,
    pub title: String,
    pub kind: wiki::Kind,
    pub body: String,
}

/// Everything one review answer asked for.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Review {
    pub edits: Vec<Edit>,
    pub patterns: Vec<PatternEdit>,
    /// Pattern ids the review cited as motivating its edits — including ones already
    /// in the wiki, which is how a third sighting gets credited for the fix.
    pub because: Vec<String>,
    pub note: String,
}

/// Every `<tag ...>body</tag>` block, as (attribute text, body).
///
/// The answer is prose from a model, not XML. An unterminated block at the end of a
/// truncated answer is the common failure and is dropped — writing half a file over
/// a working skill is far worse than writing nothing.
pub(in crate::core::evolution) fn blocks(raw: &str, tag: &str) -> Vec<(String, String)> {
    let open = format!("<{tag}");
    let close = format!("</{tag}>");
    let mut out = Vec::new();
    let mut rest = raw;
    while let Some(start) = rest.find(&open) {
        let after = &rest[start + open.len()..];
        // The tag name has to end here, or `<pattern` would also match `<patterns>`.
        let ends_here = after.starts_with('>') || after.starts_with(char::is_whitespace);
        let Some(gt) = after.find('>') else { break };
        let body_start = gt + 1;
        let Some(end) = after[body_start..].find(&close) else { break };
        if ends_here {
            out.push((
                after[..gt].trim().to_string(),
                after[body_start..body_start + end].trim_start_matches('\n').to_string(),
            ));
        }
        rest = &after[body_start + end..];
    }
    out
}

/// `name="value"` out of a block's attribute text.
pub(in crate::core::evolution) fn attr(attrs: &str, name: &str) -> Option<String> {
    let key = format!("{name}=\"");
    let start = attrs.find(&key)? + key.len();
    let end = attrs[start..].find('"')?;
    Some(attrs[start..start + end].trim().to_string())
}
