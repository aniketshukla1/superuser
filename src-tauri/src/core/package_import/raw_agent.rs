use super::*;

/// Bigger than any real package file, small enough that nothing is buried past reading.
pub(super) const MAX_FILE_BYTES: u64 = 256 * 1024;

/// A package offering more bots than this is not something anyone reviews.
pub(super) const MAX_AGENTS: usize = 12;

pub(super) const MAX_SKILLS: usize = 40;

/// What an imported assistant may use: the set a new assistant made here starts with.
pub(super) const IMPORTED_TOOLS: &[&str] = &["recall_memory", "web", "files", "todo", "connectors"];

/// A package's authoring skills and ours. The same job, so a bot that held theirs gets ours,
/// and the grant that makes ours work.
pub(super) const SAME_JOB: &[(&str, &str)] = &[("bot-creator", "persona-creator"), ("skill-creator", "skill-creator")];

#[derive(Deserialize, Default)]
pub(super) struct RawAgents {
    #[serde(default)]
    pub(super) agents: Vec<RawAgent>,
}

#[derive(Deserialize, Default)]
pub(super) struct RawAgent {
    #[serde(default)]
    pub(super) id: String,
    #[serde(default)]
    pub(super) name: String,
    #[serde(default)]
    pub(super) title: String,
    #[serde(default)]
    pub(super) role_description: String,
    #[serde(default, rename = "type")]
    pub(super) kind: String,
    #[serde(default)]
    pub(super) system_prompt: String,
    #[serde(default)]
    pub(super) skills: Vec<String>,
    #[serde(default)]
    pub(super) endpoint: String,
}

#[derive(Deserialize, Default)]
pub(super) struct RawSkills {
    #[serde(default)]
    pub(super) skills: Vec<RawSkill>,
}

#[derive(Deserialize, Default)]
pub(super) struct RawSkill {
    #[serde(default)]
    pub(super) slug: String,
    #[serde(default)]
    pub(super) title: String,
    #[serde(default)]
    pub(super) summary: String,
    #[serde(default)]
    pub(super) instructions: String,
    #[serde(default)]
    pub(super) tools: Vec<String>,
}

#[derive(Deserialize, Default)]
pub(super) struct RawSources {
    #[serde(default)]
    pub(super) sources: Vec<RawSource>,
}

#[derive(Deserialize, Default)]
pub(super) struct RawSource {
    #[serde(default, rename = "type")]
    pub(super) kind: String,
}

#[derive(Deserialize, Default)]
pub(super) struct RawModel {
    #[serde(default)]
    pub(super) model: RawModelInner,
}

#[derive(Deserialize, Default)]
pub(super) struct RawModelInner {
    #[serde(default)]
    pub(super) provider: String,
}

/// One skill as it would arrive. `problems` non-empty means it won't.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PlannedSkill {
    pub name: String,
    pub title: String,
    pub description: String,
    pub instructions: String,
    /// Tool references that name a tool this app has.
    pub tools: Vec<String>,
    /// References that don't — usually a connected app's tool, reached here through
    /// Connected apps rather than declared on a skill.
    pub unmapped: Vec<String>,
    pub problems: Vec<String>,
}

/// One assistant as it would arrive. `problems` non-empty means it won't.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PlannedPersona {
    pub id: String,
    pub name: String,
    pub tagline: String,
    pub instructions: String,
    pub tools: Vec<String>,
    pub skills: Vec<String>,
    /// The remote agent that runs its turns, for a bot that was one.
    pub remote: Option<String>,
    pub problems: Vec<String>,
    /// What changed on the way, said so it isn't discovered later.
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Plan {
    /// Of every file read. The import refuses a folder that no longer matches.
    pub digest: String,
    pub personas: Vec<PlannedPersona>,
    pub skills: Vec<PlannedSkill>,
    /// Everything in the package that has no home here, one sentence each.
    pub left_behind: Vec<String>,
}

/// What an import wrote.
#[derive(Debug, Clone, Serialize)]
pub struct Applied {
    /// The skills it installed, each switched off until the person turns it on in Settings ▸ Skills.
    pub skills: Vec<String>,
    /// Cards for the assistants, each switched off until the person turns it on.
    pub consent: Vec<crate::core::personas_install::Installed>,
}

pub(super) fn read_file(dir: &Path, name: &str, hasher: &mut Sha256) -> Result<Option<String>, String> {
    let path = dir.join(name);
    let Ok(meta) = std::fs::symlink_metadata(&path) else { return Ok(None) };
    // A link could point anywhere on this machine; a package is the folder that was picked.
    if !meta.file_type().is_file() {
        return Err(format!("`{name}` isn't a plain file in that folder."));
    }
    if meta.len() > MAX_FILE_BYTES {
        return Err(format!("`{name}` is larger than a package file should be ({} KB).", meta.len() / 1024));
    }
    let text = std::fs::read_to_string(&path).map_err(|e| format!("couldn't read `{name}`: {e}"))?;
    hasher.update(name.as_bytes());
    hasher.update([0u8]);
    hasher.update(text.as_bytes());
    hasher.update([0u8]);
    Ok(Some(text))
}

/// `${NAME:-default}` → `default`; `${NAME}` → nothing; anything else as written. The
/// environment is never consulted — see the module note.
pub fn without_environment(raw: &str) -> String {
    let t = raw.trim();
    if let Some(inner) = t.strip_prefix("${").and_then(|r| r.strip_suffix('}')) {
        return inner.split_once(":-").map(|(_, d)| d.trim().to_string()).unwrap_or_default();
    }
    t.to_string()
}

pub(super) fn one_line(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}
