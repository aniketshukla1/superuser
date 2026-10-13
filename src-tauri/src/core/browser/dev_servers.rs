use super::*;

/// Read the workspace's `.claude/launch.json` so the browser's start page can offer
/// to run the project instead of opening a search engine nobody asked for.
///
/// Missing or malformed file → no servers, not an error: most workspaces don't have
/// one, and the start page reads fine with an empty list.
pub fn dev_servers(workspace: &std::path::Path) -> Vec<DevServer> {
    let raw = match std::fs::read_to_string(workspace.join(".claude/launch.json")) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let json: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let Some(configs) = json.get("configurations").and_then(|c| c.as_array()) else {
        return Vec::new();
    };

    configs
        .iter()
        .filter_map(|c| {
            let name = c.get("name")?.as_str()?.to_string();
            let port = c.get("port").and_then(|p| p.as_u64()).and_then(|p| u16::try_from(p).ok());
            // An entry may declare an explicit url (e.g. https, or a subdomain host);
            // otherwise it's localhost on the declared port.
            let url = match c.get("url").and_then(|u| u.as_str()) {
                Some(u) => u.to_string(),
                None => format!("http://localhost:{}", port?),
            };
            // runtimeExecutable + runtimeArgs is the launch.json shape. An entry with
            // neither is an attach-only config — nothing to run, so nothing to offer.
            let exe = c.get("runtimeExecutable").and_then(|e| e.as_str())?;
            let args: Vec<String> = c
                .get("runtimeArgs")
                .and_then(|a| a.as_array())
                .map(|a| a.iter().filter_map(|v| v.as_str().map(shell_quote)).collect())
                .unwrap_or_default();
            let command = if args.is_empty() { exe.to_string() } else { format!("{exe} {}", args.join(" ")) };
            Some(DevServer { name, command, port, url })
        })
        .collect()
}

/// The command is typed into the user's shell, so an argument carrying spaces or
/// shell metacharacters has to survive as one word.
pub(super) fn shell_quote(s: &str) -> String {
    if !s.is_empty() && s.chars().all(|c| c.is_alphanumeric() || "-_./=:@".contains(c)) {
        return s.to_string();
    }
    format!("'{}'", s.replace('\'', r#"'\''"#))
}
