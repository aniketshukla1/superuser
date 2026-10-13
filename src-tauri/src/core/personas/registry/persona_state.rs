use super::super::*;

pub(in crate::core::personas) const BUILTIN_ASSISTANT: &str = include_str!("../../personas_builtin/assistant.md");

pub(in crate::core::personas) const BUILTIN_CODER: &str = include_str!("../../personas_builtin/coder.md");

pub(in crate::core::personas) const BUILTIN_OPS: &str = include_str!("../../personas_builtin/ops.md");

pub(in crate::core::personas) const BUILTIN_SECURITY: &str = include_str!("../../personas_builtin/security.md");

pub(in crate::core::personas) const BUILTIN_CLOUD_POSTURE: &str = include_str!("../../personas_builtin/cloud-posture.md");

pub(in crate::core::personas) const BUILTIN_DEP_AUDIT: &str = include_str!("../../personas_builtin/dep-audit.md");

pub(in crate::core::personas) const BUILTIN_LEAD: &str = include_str!("../../personas_builtin/lead.md");

pub(in crate::core::personas) const BUILTIN_QA: &str = include_str!("../../personas_builtin/qa.md");

pub(in crate::core::personas) const BUILTIN_WRITER: &str = include_str!("../../personas_builtin/writer.md");

pub(in crate::core::personas) const BUILTIN_ANALYST: &str = include_str!("../../personas_builtin/analyst.md");

pub(in crate::core::personas) const BUILTIN_PM: &str = include_str!("../../personas_builtin/pm.md");

pub(in crate::core::personas) const BUILTIN_FINANCE: &str = include_str!("../../personas_builtin/finance.md");

pub(in crate::core::personas) const BUILTIN_HR: &str = include_str!("../../personas_builtin/hr.md");

pub(in crate::core::personas) const BUILTIN_SUPPORT: &str = include_str!("../../personas_builtin/support.md");

pub(in crate::core::personas) const BUILTIN_EMAIL: &str = include_str!("../../personas_builtin/email.md");

pub(in crate::core::personas) const BUILTIN_EXPENSES: &str = include_str!("../../personas_builtin/expenses.md");

pub(in crate::core::personas) const BUILTIN_CHIEF: &str = include_str!("../../personas_builtin/chief.md");

pub(in crate::core::personas) const BUILTIN_REPRO: &str = include_str!("../../personas_builtin/repro.md");

pub(in crate::core::personas) const BUILTIN_PERF: &str = include_str!("../../personas_builtin/perf.md");

pub(in crate::core::personas) fn builtins() -> Vec<PersonaManifest> {
    [
        ("assistant", BUILTIN_ASSISTANT),
        ("coder", BUILTIN_CODER),
        ("ops", BUILTIN_OPS),
        ("security", BUILTIN_SECURITY),
        ("cloud-posture", BUILTIN_CLOUD_POSTURE),
        ("dep-audit", BUILTIN_DEP_AUDIT),
        ("lead", BUILTIN_LEAD),
        ("qa", BUILTIN_QA),
        ("writer", BUILTIN_WRITER),
        ("analyst", BUILTIN_ANALYST),
        ("pm", BUILTIN_PM),
        ("finance", BUILTIN_FINANCE),
        ("hr", BUILTIN_HR),
        ("support", BUILTIN_SUPPORT),
        ("email", BUILTIN_EMAIL),
        ("expenses", BUILTIN_EXPENSES),
        ("chief", BUILTIN_CHIEF),
        ("repro", BUILTIN_REPRO),
        ("perf", BUILTIN_PERF),
    ]
        .into_iter()
        .map(|(id, txt)| {
            parse_manifest(txt, Some(id), true, Some(format!("builtin:{id}")))
                .unwrap_or_else(|e| panic!("built-in persona '{id}' failed to parse: {e}"))
        })
        .collect()
}

/// Full status for the Personas settings panel.
#[derive(Debug, Clone, Serialize)]
pub struct PersonaListItem {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub tagline: String,
    pub description: String,
    /// First-person greeting + opening prompts for a fresh session (see SessionIntro).
    pub intro: String,
    pub starters: Vec<String>,
    pub family: String,
    pub kind: String,
    pub workspace: String,
    pub needs_workspace: bool,
    pub tools: Vec<String>,
    /// The worker types this assistant staffs. Empty = whatever is installed.
    pub subagents: Vec<String>,
    /// The skills it was given. Empty = everything installed.
    pub skills: Vec<String>,
    pub messaging: bool,
    pub connectors: bool,
    pub recommends: Vec<Recommendation>,
    pub builtin: bool,
    pub enabled: bool,
    pub surfaced: bool,
    pub default: bool,
}

/// One entry in the new-session assistant picker.
#[derive(Debug, Clone, Serialize)]
pub struct PersonaPickerItem {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub tagline: String,
    pub family: String,
    pub needs_workspace: bool,
    pub default: bool,
}

#[derive(Default, Serialize, Deserialize)]
pub(in crate::core::personas) struct PersonaState {
    #[serde(default)]
    pub(in crate::core::personas) enabled: HashMap<String, bool>,
    #[serde(default)]
    pub(in crate::core::personas) surfaced: HashMap<String, bool>,
    pub(in crate::core::personas) default: Option<String>,
    /// Per-assistant permission-mode override.
    ///
    /// A built-in's manifest is compiled in, so the only way to let someone change
    /// how far a shipped assistant may act is to layer their choice over it. The
    /// manifest stays the default; this is the user disagreeing with it.
    #[serde(default)]
    pub(in crate::core::personas) mode: HashMap<String, String>,
    /// Per-assistant worker roster override, same reasoning.
    #[serde(default)]
    pub(in crate::core::personas) subagents: HashMap<String, Vec<String>>,
    /// Per-assistant "who this one may ask" — the handoff grant. Ids, in
    /// the order the person picked them; absent means nobody, which is the right default
    /// for a grant.
    #[serde(default)]
    pub(in crate::core::personas) asks: HashMap<String, Vec<String>>,
    /// Per-assistant "has its own browser", same reasoning again: the grant can't be
    /// written into a manifest compiled into the binary, and it is exactly the kind of
    /// choice a person makes about one assistant and not another.
    #[serde(default)]
    pub(in crate::core::personas) browser: HashMap<String, bool>,
    /// Per-assistant "runs on a remote agent" — the AG-UI endpoint whose agent does this
    /// assistant's turns. Absent means it runs here, as every
    /// assistant does. The token for it is a secret and lives with the other secrets.
    #[serde(default)]
    pub(in crate::core::personas) remote: HashMap<String, String>,
}

pub(in crate::core::personas) fn apply_browser_grant(m: &mut PersonaManifest, on: bool) {
    let has = m.tools.iter().any(|t| t == "browser");
    if on && !has {
        m.tools.push("browser".into());
    } else if !on && has {
        m.tools.retain(|t| t != "browser");
    }
}

/// Installed personas + their lifecycle (enabled / surfaced / default). Built-ins
/// ship enabled; user-installed personas land disabled+unsurfaced pending consent.
pub struct PersonaRegistry {
    pub(in crate::core::personas) data_dir: PathBuf,
    pub(in crate::core::personas) order: Vec<String>,
    pub(in crate::core::personas) entries: HashMap<String, PersonaManifest>,
    pub(in crate::core::personas) state: PersonaState,
}
