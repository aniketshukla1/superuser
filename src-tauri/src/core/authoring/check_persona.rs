use super::*;

/// A name a file can be called and a person can type.
pub fn is_slug(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
        && s.chars().next().is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
}

/// What a skill is, as the model proposes it.
#[derive(Debug, Clone, Default)]
pub struct SkillDraft {
    pub name: String,
    pub description: String,
    pub instructions: String,
    pub tools: Vec<String>,
    /// Set when the author means to replace a skill of the same name.
    pub replace: bool,
}

/// What an assistant is, as the model proposes it.
#[derive(Debug, Clone, Default)]
pub struct PersonaDraft {
    pub id: String,
    pub name: String,
    pub tagline: String,
    pub instructions: String,
    pub tools: Vec<String>,
    pub skills: Vec<String>,
    pub mode: String,
    pub family: String,
}

/// The shortest instructions that are still instructions. Below this a skill is a
/// sentence, and a sentence belongs in the answer rather than in a file every later run
/// loads.
pub(super) const MIN_INSTRUCTIONS: usize = 120;

/// What is wrong with this skill, in sentences for whoever wrote it. Empty means it is
/// ready to put in front of the person.
pub fn check_skill(store: &SkillStore, draft: &SkillDraft) -> Vec<String> {
    let mut out = Vec::new();
    if !is_slug(&draft.name) {
        out.push(format!(
            "“{}” isn't a usable skill name. Use lowercase letters, digits and hyphens — `weekly-ops-review`.",
            draft.name.chars().take(60).collect::<String>()
        ));
    } else if store.get(&draft.name).is_some() && !draft.replace {
        out.push(format!(
            "A skill called `{}` already exists. Read it first — if yours replaces it, say so; otherwise pick another name.",
            draft.name
        ));
    }
    if draft.description.trim().is_empty() {
        out.push("Give it a description: one line saying when to reach for it. That line is the whole index a model picks from.".into());
    } else if draft.description.trim().len() > 200 {
        out.push("The description is longer than a line. Say when to use this skill in one sentence; the detail belongs in the instructions.".into());
    }
    let body = draft.instructions.trim();
    if body.is_empty() {
        out.push("The instructions are empty. Write what to do, in the imperative, as numbered steps.".into());
    } else if body.len() < MIN_INSTRUCTIONS {
        out.push(
            "The instructions are a sentence, not a skill. Say what to do step by step, what to check, and what the finished thing looks like."
                .into(),
        );
    }
    let known = tool_names();
    for t in &draft.tools {
        if !known.iter().any(|k| k == t) {
            out.push(format!("There's no tool called `{t}`. Call list_skill_tools to see the names you can declare."));
        }
    }
    out
}

/// What is wrong with this assistant. The last check composes the manifest and parses it
/// with the app's own parser, so a draft that gets past this is one the registry accepts —
/// the alternative is two validators that drift.
pub fn check_persona(reg: &PersonaRegistry, store: &SkillStore, draft: &PersonaDraft) -> Vec<String> {
    let mut out = Vec::new();
    if !is_slug(&draft.id) {
        out.push(format!(
            "“{}” isn't a usable id. Use lowercase letters, digits and hyphens — `sql-reviewer`.",
            draft.id.chars().take(60).collect::<String>()
        ));
    } else if reg.manifest(&draft.id).is_some() {
        out.push(format!("There's already an assistant with the id `{}`. Read it first, then pick another id.", draft.id));
    }
    if draft.name.trim().is_empty() {
        out.push("Give it a name — what the person will call it.".into());
    }
    if draft.tagline.trim().is_empty() {
        out.push("Give it a tagline: one line saying what it is for, shown wherever it is listed.".into());
    }
    if draft.instructions.trim().len() < MIN_INSTRUCTIONS {
        out.push(
            "The instructions are too thin to be a standing role. Say what it does, how it works, what it refuses, and what a good answer from it looks like."
                .into(),
        );
    }
    for t in &draft.tools {
        if !GRANTABLE_TOOLS.contains(&t.as_str()) {
            out.push(format!(
                "`{t}` isn't something an assistant can be given. The list is: {}.",
                GRANTABLE_TOOLS.join(", ")
            ));
        }
    }
    let installed = store.names();
    for s in &draft.skills {
        if !installed.iter().any(|n| n == s) {
            out.push(format!("There's no skill called `{s}`. Call list_persona_skills to see what this machine has."));
        }
    }
    // The app's own parser has the last word: modes, families, ids and tool names are its
    // rules, and a second copy of them here would be a second set to keep in step.
    if out.is_empty() {
        if let Err(e) = personas::parse_manifest(&manifest_text(draft), Some(&draft.id), false, None) {
            out.push(format!("The assistant didn't come out valid: {e}"));
        }
    }
    out
}

/// The manifest as it will be written. YAML built by hand is a hazard for values a person
/// typed, so every field that could carry a colon or a newline goes through the serializer.
pub fn manifest_text(draft: &PersonaDraft) -> String {
    let mut fm = serde_yaml_ng::Mapping::new();
    let mut put = |k: &str, v: serde_yaml_ng::Value| {
        fm.insert(serde_yaml_ng::Value::String(k.into()), v);
    };
    put("id", draft.id.clone().into());
    put("name", draft.name.trim().into());
    put("tagline", draft.tagline.trim().into());
    let family = if draft.family.trim().is_empty() { "knowledge" } else { draft.family.trim() };
    put("family", family.into());
    put("kind", "assistant".into());
    let mode = if draft.mode.trim().is_empty() { "interactive" } else { draft.mode.trim() };
    put("default_permission_mode", mode.into());
    put("tools", draft.tools.clone().into());
    put("skills", draft.skills.clone().into());
    put("created", chrono::Local::now().format("%Y-%m-%d").to_string().into());
    let yaml = serde_yaml_ng::to_string(&fm).unwrap_or_default();
    format!("---\n{yaml}---\n\n{}\n", draft.instructions.trim())
}

/// Write the assistant, and switch it on: the person pressed the card, which is the
/// consent a user-installed assistant otherwise waits for.
pub fn save_persona(data_dir: &Path, draft: &PersonaDraft) -> Result<PathBuf, String> {
    if !is_slug(&draft.id) {
        return Err(format!("'{}' isn't a usable id", draft.id));
    }
    let dir = data_dir.join("personas");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("{}.md", draft.id));
    crate::core::store::write_atomic(&path, &manifest_text(draft)).map_err(|e| e.to_string())?;
    let mut reg = PersonaRegistry::load(data_dir);
    reg.set_enabled(&draft.id, true)?;
    Ok(path)
}

/// Every tool name a skill may declare, sorted, for `list_skill_tools` and for checking a
/// draft against.
pub fn tool_names() -> Vec<String> {
    let mut names: Vec<String> = crate::core::agent::every_tool().into_iter().map(|(_, n)| n).collect();
    names.push("ask_coworker".into());
    names.push("load_skill".into());
    names.sort();
    names.dedup();
    names
}

/// The tools, grouped by the capability that carries them — a model choosing what a skill
/// may use is choosing capabilities, and a flat list of ninety names hides that.
pub fn tools_by_capability() -> Vec<(String, Vec<String>)> {
    let mut out: Vec<(String, Vec<String>)> = Vec::new();
    for (cap, name) in crate::core::agent::every_tool() {
        match out.iter_mut().find(|(c, _)| c == cap) {
            Some((_, list)) => list.push(name),
            None => out.push((cap.to_string(), vec![name])),
        }
    }
    out
}
