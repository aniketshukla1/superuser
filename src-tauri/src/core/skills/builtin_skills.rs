use super::*;

#[derive(Debug, Clone, Serialize)]
pub struct Skill {
    pub name: String,
    pub description: String,
    /// Full body — the instructions loaded on demand.
    pub instructions: String,
    pub path: Option<String>,
    pub allowed_tools: Vec<String>,
}

/// Catalog line for the Skills tab + the session-start injection (name + description).
#[derive(Debug, Clone, Serialize)]
pub struct SkillCard {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Default, Deserialize)]
pub(super) struct RawSkill {
    pub(super) name: Option<String>,
    pub(super) description: Option<String>,
    #[serde(rename = "allowed-tools", alias = "allowed_tools", default)]
    pub(super) allowed_tools: Option<StrOrVec>,
}

/// `allowed-tools` is either a YAML list or a comma-separated string (Anthropic
/// SKILL.md commonly uses the string form).
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(super) enum StrOrVec {
    One(String),
    Many(Vec<String>),
}

impl StrOrVec {
    pub(super) fn into_vec(self) -> Vec<String> {
        match self {
            StrOrVec::One(s) => s.split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect(),
            StrOrVec::Many(v) => v.into_iter().map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect(),
        }
    }
}

pub(super) fn split_frontmatter(text: &str) -> Result<(&str, String), String> {
    let t = text.strip_prefix('\u{feff}').unwrap_or(text);
    if !t.starts_with("---") {
        return Err("SKILL.md has to open with a `---` YAML frontmatter block".into());
    }
    let after = &t[3..];
    let end = after.find("\n---").ok_or("frontmatter never closes (no second `---`)")?;
    Ok((&after[..end], after[end + 4..].trim_start_matches('\n').to_string()))
}

pub fn parse_skill(text: &str, fallback_name: &str, path: Option<String>) -> Result<Skill, String> {
    let (yaml, body) = split_frontmatter(text)?;
    let raw: RawSkill = serde_yaml_ng::from_str(yaml).map_err(|e| format!("invalid SKILL.md frontmatter: {e}"))?;
    let name = raw
        .name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(fallback_name)
        .to_string();
    if name.is_empty() {
        return Err("skill needs a name (or a folder to derive one from)".into());
    }
    if body.trim().is_empty() {
        return Err(format!("skill '{name}' has no instructions (the body)"));
    }
    Ok(Skill {
        name,
        description: raw.description.unwrap_or_default().trim().to_string(),
        instructions: body.trim().to_string(),
        path,
        allowed_tools: raw.allowed_tools.map(StrOrVec::into_vec).unwrap_or_default(),
    })
}

/// Deliverable templates we ship. Each produces an actual file (a CSV, a document)
/// rather than a chat answer — that is the whole point of a skill. A user skill of
/// the same name wins, so these can be overridden by editing data_dir/skills/.
pub(super) const BUILTIN_SKILLS: &[(&str, &str)] = &[
    ("financial-budget", include_str!("../skills_builtin/financial-budget.md")),
    ("project-tracker", include_str!("../skills_builtin/project-tracker.md")),
    ("meeting-brief", include_str!("../skills_builtin/meeting-brief.md")),
    ("research-brief", include_str!("../skills_builtin/research-brief.md")),
    // Security. These produce a review rather than a spreadsheet, but the shape is
    // the same: a procedure the assistant follows to the end, including the part
    // where it says what it could not check.
    ("semgrep-review", include_str!("../skills_builtin/semgrep-review.md")),
    ("secret-scan", include_str!("../skills_builtin/secret-scan.md")),
    ("security-fix-pr", include_str!("../skills_builtin/security-fix-pr.md")),
    ("iac-scan", include_str!("../skills_builtin/iac-scan.md")),
    ("aws-posture", include_str!("../skills_builtin/aws-posture.md")),
    ("dependency-audit", include_str!("../skills_builtin/dependency-audit.md")),
    ("safe-upgrade-pr", include_str!("../skills_builtin/safe-upgrade-pr.md")),
    // Engineering. Procedures rather than job titles: "backend engineering" is not a
    // thing anyone does, whereas designing an endpoint and planning a migration are.
    // Making more of the app from inside it: both interview first, and
    // both end at a card the person presses.
    ("skill-creator", include_str!("../skills_builtin/skill-creator.md")),
    ("persona-creator", include_str!("../skills_builtin/persona-creator.md")),
    ("code-review", include_str!("../skills_builtin/code-review.md")),
    ("debug-repro", include_str!("../skills_builtin/debug-repro.md")),
    ("api-design", include_str!("../skills_builtin/api-design.md")),
    ("db-migration", include_str!("../skills_builtin/db-migration.md")),
    ("refactor-safely", include_str!("../skills_builtin/refactor-safely.md")),
    ("test-backfill", include_str!("../skills_builtin/test-backfill.md")),
    ("ui-component", include_str!("../skills_builtin/ui-component.md")),
    ("frontend-a11y", include_str!("../skills_builtin/frontend-a11y.md")),
    ("design-review", include_str!("../skills_builtin/design-review.md")),
    // Quality.
    ("test-plan", include_str!("../skills_builtin/test-plan.md")),
    ("exploratory-charter", include_str!("../skills_builtin/exploratory-charter.md")),
    ("bug-report", include_str!("../skills_builtin/bug-report.md")),
    ("staging-repro", include_str!("../skills_builtin/staging-repro.md")),
    // Performance.
    ("perf-investigation", include_str!("../skills_builtin/perf-investigation.md")),
    // Recruiting.
    ("talent-sourcing", include_str!("../skills_builtin/talent-sourcing.md")),
    // Documentation.
    ("how-to-guide", include_str!("../skills_builtin/how-to-guide.md")),
    ("api-reference", include_str!("../skills_builtin/api-reference.md")),
    ("release-notes", include_str!("../skills_builtin/release-notes.md")),
    // Data and product.
    ("data-profile", include_str!("../skills_builtin/data-profile.md")),
    ("analysis-brief", include_str!("../skills_builtin/analysis-brief.md")),
    ("product-spec", include_str!("../skills_builtin/product-spec.md")),
    ("decision-record", include_str!("../skills_builtin/decision-record.md")),
    // Finance. `financial-budget` above predates this group; these are the rest of
    // the same job — where the money went, and when it runs out.
    ("variance-report", include_str!("../skills_builtin/variance-report.md")),
    ("runway-model", include_str!("../skills_builtin/runway-model.md")),
    // People.
    ("job-description", include_str!("../skills_builtin/job-description.md")),
    ("interview-loop", include_str!("../skills_builtin/interview-loop.md")),
    ("onboarding-plan", include_str!("../skills_builtin/onboarding-plan.md")),
    // Support.
    ("support-triage", include_str!("../skills_builtin/support-triage.md")),
    ("help-article", include_str!("../skills_builtin/help-article.md")),
];

/// The installed skills, discovered from `data_dir/skills/<name>/SKILL.md`, and which of them
/// are switched off.
pub struct SkillStore {
    pub(super) skills: BTreeMap<String, Skill>,
    pub(super) off: BTreeSet<String>,
    pub(super) data_dir: PathBuf,
}

/// The skills switched off, by name — `skills.json`, beside `personas.json`.
///
/// A skill is on unless it is named here: installing one never touches this file, and every
/// skill installed before it existed stays as it was. Off is app-wide — installed and listed,
/// and offered to no assistant: not in a catalogue, not in the composer's menu, not loadable.
/// Muting a skill for one conversation is `session_skills`, a different switch. A package
/// import writes its skills in switched off, the way its assistants arrive, so pressing Import
/// no longer puts them in every generalist's catalogue at once.
#[derive(Debug, Default, Serialize, Deserialize)]
pub(super) struct SkillState {
    #[serde(default)]
    pub(super) off: BTreeSet<String>,
}

pub(super) fn state_path(data_dir: &Path) -> PathBuf {
    data_dir.join("skills.json")
}

pub(super) fn read_state(data_dir: &Path) -> SkillState {
    fs::read_to_string(state_path(data_dir)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

pub(super) fn write_state(data_dir: &Path, state: &SkillState) -> Result<(), String> {
    let json = serde_json::to_string_pretty(state).map_err(|e| e.to_string())?;
    crate::core::store::write_atomic(&state_path(data_dir), &json).map_err(|e| format!("couldn't save the switch: {e}"))
}
