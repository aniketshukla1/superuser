use super::*;

/// Risk classes a tool can produce — drives approval-gating and the install-consent
/// screen. Ordered least→most sensitive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskClass {
    Read,
    WriteLocal,
    Exec,
    External,
}

/// Tools that stay in the context window. Everything else is discovered on demand
/// through Anthropic's tool search (HARNESS-PLAN §2.13).
///
/// Chosen by measurement, not by taste. The full catalogue is 53 tools and ~6,490
/// tokens of schema on every request, whether the turn needs `slides_edit` or not.
/// (The plan says 10.2k; measured here it is 6,490 — the number this file's own test
/// prints, and the one every reduction is judged against.)
///
/// This list is the loop: read, write, edit, look around, search, run something, check
/// the repo, keep the todo, propose a plan, ask a question, reach the web. The long
/// tail — the office formats, the board, connectors, knowledge, scheduling, toolchain
/// probes, subagents — is real work that most turns never touch, and the doc's own
/// advice is to keep 3–5 frequently used tools resident. Fourteen is more than that
/// because this app's core loop genuinely is wider than five.
///
/// MCP tools are deliberately absent: they are the most variable part of the list and
/// therefore the most cache-hostile, so they defer by default.
pub(super) const RESIDENT: &[&str] = &[
    "read_file", "write_file", "edit_file", "list_dir",
    "grep", "run_shell", "recall_output",
    "git_status", "git_diff", "git_log",
    "todo_write", "propose_plan", "ask_user",
    "web_search", "web_fetch",
];

/// Whether this tool is sent up front rather than found by searching for it.
pub fn is_resident(name: &str) -> bool {
    RESIDENT.contains(&name)
}

/// A session-context prerequisite a capability may require.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Req {
    Workspace,
    Executor,
    Todo,
}

/// What the current session provides — capability availability is checked against it.
#[derive(Debug, Default, Clone)]
pub struct AgentContext {
    pub workspace: Option<PathBuf>,
    pub roots: Vec<PathBuf>,
    pub has_executor: bool,
    pub has_todo: bool,
}

impl AgentContext {
    pub(super) fn satisfies(&self, r: Req) -> bool {
        match r {
            Req::Workspace => self.workspace.is_some(),
            Req::Executor => self.has_executor,
            Req::Todo => self.has_todo,
        }
    }
}

/// A concrete tool offered to the model (an OpenAI-style function tool).
#[derive(Debug, Clone, Serialize)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    /// JSON Schema for the tool's parameters.
    pub schema: Value,
    pub risk: RiskClass,
    /// The catalog capability that contributed this tool.
    pub capability: String,
}

pub(super) fn tool(name: &str, description: &str, capability: &str, risk: RiskClass, schema: Value) -> ToolSpec {
    ToolSpec { name: name.into(), description: description.into(), schema, risk, capability: capability.into() }
}

pub(super) fn params(properties: Value, required: &[&str]) -> Value {
    json!({ "type": "object", "properties": properties, "required": required })
}

/// A vetted bundle of tools behind a stable id, plus its context prerequisites and
/// the risk classes it can produce.
pub struct Capability {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub(super) requires: &'static [Req],
    pub risk: &'static [RiskClass],
    pub(super) build: fn() -> Vec<ToolSpec>,
}

impl Capability {
    pub(super) fn available(&self, ctx: &AgentContext) -> bool {
        self.requires.iter().all(|r| ctx.satisfies(*r))
    }
}
