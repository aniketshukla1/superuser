use super::*;

pub(super) const README: &str = r#"# Subagent types

A subagent does one self-contained task in its own context and hands back only the
conclusion — the pages it read and commands it ran never enter your conversation.

A *type* decides two things: the prompt it works under, and the tools it may use.
Two are built in:

- `general-purpose` — investigation and action: search, read, run commands, write files
- `explore` — read-only research; cannot change anything

Add your own by dropping a `.md` file in this folder. Copy `example.md.template`
to `<name>.md` to start. Files are re-read every turn, so a new type works on your
next message — no restart.

The `tools:` line is the real constraint: a type listing only `read_file, ls,
search_files` cannot modify anything, whatever its prompt says or its task asks
for. Omit the line to inherit everything the main agent has. Some tools are never
available to a subagent whatever you list: sending messages, driving the desktop,
syncing connectors, and spawning further subagents.
"#;

pub(super) const EXAMPLE: &str = r#"---
name: research-report
description: Research one topic across many sources and return a short report with citations. Use when answering would mean opening a lot of pages and only the conclusion matters.
tools: web_search, web_fetch, read_file
---
You are a research assistant. You receive one topic and return one report.

How to work:
1. Search broadly first, then follow the two or three most promising sources.
2. Prefer primary sources (official docs, the vendor's own pricing page, the original
   announcement) over articles describing them.
3. Cross-check every number, date and price against a second source.

Keep the report under 400 words, containing in this order:
- Answer: two or three sentences that settle the question
- Findings: bullets, each ending with the source URL
- Unconfirmed: anything you could not verify from a primary source

Write "not found" where you came up empty. Never fill a gap with a guess.
"#;

/// Create `subagents/` with a README and a template the first time, so the feature
/// is discoverable from the filesystem rather than only from documentation. Never
/// overwrites: a user's edits to these files are theirs.
pub fn ensure_templates(data_dir: &Path) {
    let dir = data_dir.join("subagents");
    if fs::create_dir_all(&dir).is_err() {
        return;
    }
    for (name, body) in [("README.md", README), ("example.md.template", EXAMPLE)] {
        let path = dir.join(name);
        if !path.exists() {
            let _ = fs::write(path, body);
        }
    }
}

#[derive(Debug, Default, Deserialize)]
pub(super) struct RawType {
    pub(super) name: Option<String>,
    pub(super) description: Option<String>,
    pub(super) tools: Option<String>,
}

/// Same frontmatter+body shape as a skill: `---\nname/description/tools\n---\nbody`.
pub fn parse_type(text: &str, fallback_name: &str) -> Result<SubagentType, String> {
    let t = text.strip_prefix('\u{feff}').unwrap_or(text);
    let rest = t.strip_prefix("---").ok_or("missing frontmatter")?;
    let end = rest.find("\n---").ok_or("unterminated frontmatter")?;
    let raw: RawType = serde_yaml_ng::from_str(rest[..end].trim()).map_err(|e| e.to_string())?;
    let body = rest[end + 4..].trim();
    if body.is_empty() {
        return Err("a subagent type needs instructions".into());
    }
    let name = raw.name.unwrap_or_else(|| fallback_name.to_string());
    if name.trim().is_empty() {
        return Err("a subagent type needs a name".into());
    }
    Ok(SubagentType {
        name: name.trim().to_string(),
        description: raw.description.unwrap_or_default().trim().to_string(),
        // An omitted `tools` inherits everything; an empty list is a type that can
        // only think, which is a legitimate (if unusual) thing to want.
        tools: raw.tools.map(|s| s.split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect()),
        instructions: body.to_string(),
        builtin: false,
    })
}

/// The tools this type actually gets: its allowlist if it has one, otherwise the
/// parent's — always minus `BLOCKED`.
pub fn allowed_tools(t: &SubagentType, parent: &[ToolSpec]) -> Vec<ToolSpec> {
    parent
        .iter()
        .filter(|s| !BLOCKED.contains(&s.name.as_str()))
        .filter(|s| t.tools.as_ref().map(|allow| allow.iter().any(|a| a == &s.name)).unwrap_or(true))
        .cloned()
        .collect()
}
