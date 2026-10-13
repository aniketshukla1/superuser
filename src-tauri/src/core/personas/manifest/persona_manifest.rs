use super::super::*;

#[derive(Debug, Clone, Serialize)]
pub struct Recommendation {
    pub kind: String, // "connector" | "mcp"
    pub reference: String,
    pub reason: String,
    pub tier: String, // "core" | "optional"
}

#[derive(Debug, Clone, Serialize)]
pub struct PersonaManifest {
    pub id: String,
    pub name: String,
    pub system_prompt: String,
    pub icon: String,
    pub tagline: String,
    pub description: String,
    /// First-person greeting shown when you open a fresh session with this persona
    /// ("I'm finance. I can help you…"). Empty falls back to the tagline.
    pub intro: String,
    /// Concrete opening prompts offered as one-click chips beside the intro.
    pub starters: Vec<String>,
    pub tools: Vec<String>,
    pub family: String,    // "code" | "knowledge"
    /// "assistant" | "workbench". The load-bearing split, and it is about PLACE: a
    /// WORKBENCH is bound to one (a repo, a machine), so picking it flips the sidebar
    /// to the project list and gates a folder; an ASSISTANT is always available and
    /// needs no folder.
    ///
    /// This used to read "a workbench can run shell and is bound to a place", as
    /// though those were one property. They are not, and every assistant now ships
    /// with a shell (2026-09-01) while belonging nowhere near the project list. What
    /// a persona may DO is `tools`; where it LIVES is this.
    ///
    /// Deliberately NOT "built-in vs user-made" — provenance changes (we ship preset
    /// assistants), and where something lives doesn't.
    pub kind: String,
    pub workspace: String, // derived from family: code→"git", knowledge→"deliverable"
    pub messaging: bool,
    pub connectors: bool,
    pub default_permission_mode: String,
    /// This assistant's work degrades badly on a small model.
    ///
    /// It does NOT force a model. Forcing one would either override the model the
    /// user picked or push a local-only user's code to a cloud they deliberately
    /// switched off — and `local_only` is a hard egress switch, not a preference.
    /// What it does is make a degraded run say so, which is the same rule the
    /// Coverage note follows: a check that ran weakly must not read like one that
    /// ran well.
    ///
    /// This replaces `recommended_models`, a list of model ids that nothing read,
    /// nothing declared, and no local-first user could act on — being told an
    /// assistant prefers `claude-opus-4-8` is not useful when your machine runs
    /// Ollama.
    pub needs_capable_model: bool,
    pub skills: Vec<String>,
    /// The worker types this assistant staffs when it delegates.
    ///
    /// Empty means "whatever is installed", which is what every assistant did
    /// before this existed and remains the sensible default for a generalist. A
    /// LEAD is the case this is for: it should hand work to a named roster, not to
    /// whichever types happen to be on the machine.
    pub subagents: Vec<String>,
    pub mcp: Vec<String>,
    pub recommends: Vec<Recommendation>,
    pub builtin: bool,
    pub source: Option<String>,
}

impl PersonaManifest {
    pub fn needs_workspace(&self) -> bool {
        self.workspace != "none"
    }
}

#[derive(Debug, Default, Deserialize)]
pub(in crate::core::personas) struct RawManifest {
    pub(in crate::core::personas) id: Option<String>,
    pub(in crate::core::personas) name: Option<String>,
    pub(in crate::core::personas) icon: Option<String>,
    pub(in crate::core::personas) tagline: Option<String>,
    pub(in crate::core::personas) description: Option<String>,
    pub(in crate::core::personas) intro: Option<String>,
    #[serde(default)]
    pub(in crate::core::personas) starters: Vec<String>,
    #[serde(default)]
    pub(in crate::core::personas) tools: Vec<String>,
    pub(in crate::core::personas) family: Option<String>,
    pub(in crate::core::personas) kind: Option<String>,
    #[allow(dead_code)] // accepted + tolerated for older manifests; family drives workspace now
    pub(in crate::core::personas) workspace: Option<String>,
    #[serde(default)]
    pub(in crate::core::personas) messaging: bool,
    #[serde(default)]
    pub(in crate::core::personas) connectors: bool,
    pub(in crate::core::personas) default_permission_mode: Option<String>,
    #[serde(default)]
    pub(in crate::core::personas) needs_capable_model: bool,
    #[serde(default)]
    pub(in crate::core::personas) skills: Vec<String>,
    #[serde(default)]
    pub(in crate::core::personas) subagents: Vec<String>,
    #[serde(default)]
    pub(in crate::core::personas) mcp: Vec<String>,
    pub(in crate::core::personas) recommends: Option<Vec<RawRec>>,
}

#[derive(Debug, Deserialize)]
pub(in crate::core::personas) struct RawRec {
    pub(in crate::core::personas) connector: Option<String>,
    pub(in crate::core::personas) mcp: Option<String>,
    #[serde(default)]
    pub(in crate::core::personas) reason: String,
    pub(in crate::core::personas) tier: Option<String>,
}

pub(in crate::core::personas) fn slugify(stem: &str) -> String {
    let lowered = stem.trim().to_lowercase();
    let re = Regex::new(r"[^a-z0-9_-]+").unwrap();
    let dashed = re.replace_all(&lowered, "-");
    dashed.trim_matches(|c| c == '-' || c == '_').chars().take(64).collect()
}
