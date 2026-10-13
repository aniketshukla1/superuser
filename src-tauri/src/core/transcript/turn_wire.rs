use super::*;

/// The wire dialect a transcript is written in. Native replay requires an exact
/// match; anything else is projected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WireShape {
    Claude,
    OpenAi,
    Ollama,
}

impl WireShape {
    pub fn of(profile: &ModelProfile) -> Self {
        match profile.kind {
            ProviderKind::Anthropic => WireShape::Claude,
            ProviderKind::OpenAiCompatible => WireShape::OpenAi,
            // Codex runs its own agent and never reaches this loop's message
            // assembly; it is grouped with Ollama only so this function is total.
            ProviderKind::Ollama | ProviderKind::CodexCli | ProviderKind::ClaudeCli => WireShape::Ollama,
        }
    }
}

/// One turn's native message traffic, as the provider that produced it saw it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TurnWire {
    pub shape: WireShape,
    /// The model that produced it. Kept for the record and for the trace: "why is
    /// this being projected" is answered by the pair of model ids, not by the shape.
    pub model: String,
    pub messages: Vec<Value>,
}

impl TurnWire {
    /// Every tool this turn called, in order and with repeats.
    ///
    /// Reads both wire shapes, because the point of storing the native traffic is
    /// that it is native: Anthropic writes `tool_use` content blocks, OpenAI and
    /// Ollama write a `tool_calls` array, and a reader that knew only one would
    /// report "no tools ran" for half the providers.
    pub fn tool_names(&self) -> Vec<String> {
        let mut out = Vec::new();
        for m in &self.messages {
            if let Some(calls) = m["tool_calls"].as_array() {
                for c in calls {
                    if let Some(n) = c["function"]["name"].as_str().or_else(|| c["name"].as_str()) {
                        out.push(n.to_string());
                    }
                }
            }
            if let Some(blocks) = m["content"].as_array() {
                for b in blocks {
                    if b["type"] == "tool_use" {
                        if let Some(n) = b["name"].as_str() {
                            out.push(n.to_string());
                        }
                    }
                }
            }
        }
        out
    }
}

/// The most one tool result may occupy in a *stored* transcript.
///
/// Sessions are rewritten whole on every append, so an uncapped transcript makes
/// every later turn's save proportional to every earlier turn's tool output. The
/// budget squeezes tool results inside a turn already; this is the same idea applied
/// to what outlives it.
pub const RESULT_CAP: usize = 4_000;

/// The most one stored turn may occupy in total, across all of its results.
pub const TURN_CAP: usize = 24_000;
