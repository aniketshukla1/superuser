//! agent

use super::*;

/// The runtime form of a persona: its prompt + declared capabilities + traits.
#[derive(Debug, Clone, Serialize)]
pub struct Agent {
    pub name: String,
    pub title: String,
    pub system_prompt: String,
    pub needs_workspace: bool,
    pub tools: Vec<String>,
    pub family: String,
    pub messaging: bool,
    pub connectors: bool,
}

impl Agent {
    pub fn from_manifest(m: &PersonaManifest) -> Self {
        Agent {
            name: m.id.clone(),
            title: m.name.clone(),
            system_prompt: m.system_prompt.clone(),
            needs_workspace: m.needs_workspace(),
            tools: m.tools.clone(),
            family: m.family.clone(),
            messaging: m.messaging,
            connectors: m.connectors,
        }
    }

    /// The concrete tool specs this agent can use in the given session context.
    pub fn tools(&self, ctx: &AgentContext) -> Vec<ToolSpec> {
        expand(&self.tools, ctx)
    }
}
