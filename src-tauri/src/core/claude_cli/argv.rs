use super::*;

/// The only tools a delegated turn gets: reading and searching the session's folder.
pub(super) const TOOLS: &str = "Read,Glob,Grep";

/// Where `claude` is, if anywhere.
///
/// PATH first, the way a shell would look. Then the places its installers put it,
/// because an app opened from the Dock does not inherit the shell's PATH: on macOS a
/// GUI process gets `/usr/bin:/bin:/usr/sbin:/sbin`, and every one of these is outside
/// it. Without the fallbacks the provider would read "not installed" on the machine of
/// someone who uses `claude` every day.
pub fn bin() -> Option<PathBuf> {
    let on_path = std::env::var_os("PATH")
        .and_then(|p| std::env::split_paths(&p).map(|d| d.join("claude")).find(|c| c.is_file()));
    if on_path.is_some() {
        return on_path;
    }
    let home = dirs::home_dir()?;
    [
        home.join(".local/bin/claude"),
        home.join(".claude/local/claude"),
        PathBuf::from("/opt/homebrew/bin/claude"),
        PathBuf::from("/usr/local/bin/claude"),
    ]
    .into_iter()
    .find(|c| c.is_file())
}

/// Whether the CLI is installed and has been signed in, without running it.
///
/// Two file checks, as `codex_ready` does: this paints the model picker, and a
/// subprocess per repaint is a cost the answer does not need. Signed in means an
/// account recorded in the CLI's own config (`oauthAccount`, written by `claude
/// login`) or the credentials file it keeps where there is no keychain. It cannot
/// tell an expired session from a live one — `run` reports that one, in the CLI's own
/// words, on the first turn.
pub fn ready() -> bool {
    bin().is_some() && signed_in()
}

pub(super) fn signed_in() -> bool {
    let (config_json, dir) = match std::env::var_os("CLAUDE_CONFIG_DIR") {
        Some(d) => {
            let d = PathBuf::from(d);
            (d.join(".claude.json"), d)
        }
        None => match dirs::home_dir() {
            Some(h) => (h.join(".claude.json"), h.join(".claude")),
            None => return false,
        },
    };
    if dir.join(".credentials.json").is_file() {
        return true;
    }
    // A substring rather than a parse: the file is the CLI's whole state, tens of
    // kilobytes of project history, and the one key is all this needs.
    std::fs::read_to_string(config_json).map(|s| s.contains("\"oauthAccount\"")).unwrap_or(false)
}

/// The exact command line handed to `claude`.
///
/// Split out to be testable, like Codex's: an unversioned CLI contract with no compiler
/// behind it, where a missing flag does not fail to build — it fails every turn.
pub(super) fn argv(model: &str) -> Vec<String> {
    vec![
        "--print".into(),
        // One JSON event per line as it happens. Without it the CLI prints prose, and
        // only at the end. `--verbose` is required alongside it in print mode.
        "--output-format".into(),
        "stream-json".into(),
        "--verbose".into(),
        // Each turn is a fresh process with the conversation flattened into its prompt,
        // so a saved session would be clutter in the user's own `claude --resume` list.
        "--no-session-persistence".into(),
        "--model".into(),
        model.into(),
        // The tool set, and the permission to use it, are both set: `--tools` limits
        // what exists, `--allowedTools` pre-approves it, and `dontAsk` denies anything
        // else rather than asking a question no one is on stdin to answer.
        "--tools".into(),
        TOOLS.into(),
        "--allowedTools".into(),
        TOOLS.into(),
        "--permission-mode".into(),
        "dontAsk".into(),
        // No MCP servers from the user's own config. They would arrive as tools outside
        // the list above, and a delegated turn starting someone's servers is a side
        // effect they did not ask this app for.
        "--strict-mcp-config".into(),
    ]
}

/// Something worth showing the user while Claude Code is still working.
#[derive(Debug, Clone, PartialEq)]
pub enum Update {
    /// Claude said something — narration mid-turn, or the answer at the end.
    Text(String),
    /// Claude used one of its tools. `detail` is the argument a person would want to
    /// see (the file, the pattern), not the whole input object.
    Tool { name: String, detail: String },
}

/// What one line of the stream means.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Event {
    /// What one assistant message said and did, in order.
    Said(Vec<Update>),
    /// The turn's final word: `result`, success or not.
    Done { ok: bool, text: String },
    /// The CLI reported an API failure as a message (a stale login, an overload).
    Failure(String),
}

/// Turn one line of `--output-format stream-json` into an event, if it carries one.
///
/// Unknown and malformed lines are `None`, never an error: a new event type in some
/// future release should cost the user nothing.
pub(super) fn parse(line: &str) -> Option<Event> {
    let v: serde_json::Value = serde_json::from_str(line.trim()).ok()?;
    match v.get("type")?.as_str()? {
        "assistant" => {
            let content = v.get("message")?.get("content")?.as_array()?;
            // An API failure arrives dressed as an assistant message — model
            // `<synthetic>`, `error` set — before the `result` that ends the turn. It
            // must not be shown as if Claude had said it.
            if v.get("error").is_some() || v.get("is_api_error_message").and_then(|b| b.as_bool()) == Some(true) {
                let text = content.iter().filter_map(|c| c.get("text")?.as_str()).collect::<Vec<_>>().join(" ");
                let text = text.trim();
                return (!text.is_empty()).then(|| Event::Failure(text.to_string()));
            }
            // One message can carry text and a tool call together.
            let said = blocks(content);
            (!said.is_empty()).then_some(Event::Said(said))
        }
        "result" => {
            let ok = v.get("is_error").and_then(|b| b.as_bool()) != Some(true)
                && v.get("subtype").and_then(|s| s.as_str()) == Some("success");
            let text = v.get("result").and_then(|r| r.as_str()).unwrap_or("").trim().to_string();
            Some(Event::Done { ok, text })
        }
        _ => None,
    }
}

/// Every update in one assistant message's content, in order.
pub(super) fn blocks(content: &[serde_json::Value]) -> Vec<Update> {
    content
        .iter()
        .filter_map(|c| match c.get("type")?.as_str()? {
            "text" => {
                let t = c.get("text")?.as_str()?.trim();
                (!t.is_empty()).then(|| Update::Text(t.to_string()))
            }
            "tool_use" => {
                let name = c.get("name")?.as_str()?.to_string();
                let input = c.get("input");
                let detail = ["file_path", "pattern", "path", "query", "url"]
                    .iter()
                    .find_map(|k| input?.get(*k)?.as_str())
                    .unwrap_or("")
                    .to_string();
                Some(Update::Tool { name, detail })
            }
            _ => None,
        })
        .collect()
}
