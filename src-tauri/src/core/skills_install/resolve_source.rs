use super::*;

/// Cap on a fetched SKILL.md. Real ones are a few KB; anything near this is either
/// a mistake or an attempt to bury instructions past where a human will read.
pub(super) const MAX_BYTES: usize = 256 * 1024;

/// What the user is asked to approve.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Preview {
    pub name: String,
    pub description: String,
    /// The full instructions — shown in full, because this is the thing being
    /// approved and a summary of it is not what the agent will read.
    pub instructions: String,
    /// Tools the skill asks to use, if it declares any.
    pub allowed_tools: Vec<String>,
    /// Where it came from, for the record.
    pub source: String,
    /// The exact URL fetched, so the user can go and look.
    pub url: String,
    /// True when a skill of this name is already installed and would be replaced.
    pub replaces_existing: bool,
}

/// Turn what the user typed into a raw URL for a `SKILL.md`.
///
/// Accepts the shapes people actually paste:
/// * `owner/repo` → that repo's root `SKILL.md` on its default branch
/// * `owner/repo/path/to/skill` → a subdirectory
/// * a GitHub web URL (`github.com/o/r/blob/main/x/SKILL.md` or `/tree/main/x`)
/// * any `https://` URL ending in `.md`
pub fn resolve_source(source: &str) -> Result<String, String> {
    let s = source.trim().trim_end_matches('/');
    if s.is_empty() {
        return Err("give a skill source — `owner/repo`, a GitHub link, or a URL to a SKILL.md".into());
    }

    if let Some(rest) = s.strip_prefix("https://github.com/").or_else(|| s.strip_prefix("http://github.com/")) {
        let parts: Vec<&str> = rest.split('/').collect();
        if parts.len() < 2 {
            return Err(format!("'{source}' doesn't name a repository"));
        }
        let (owner, repo) = (parts[0], parts[1]);
        // .../blob/<ref>/<path> and .../tree/<ref>/<path> both appear in the address
        // bar; raw.githubusercontent wants <ref>/<path> either way.
        if parts.len() > 3 && (parts[2] == "blob" || parts[2] == "tree") {
            let tail = parts[3..].join("/");
            let tail = if tail.ends_with(".md") { tail } else { format!("{tail}/SKILL.md") };
            return Ok(format!("https://raw.githubusercontent.com/{owner}/{repo}/{tail}"));
        }
        let tail = if parts.len() > 2 { format!("{}/SKILL.md", parts[2..].join("/")) } else { "SKILL.md".into() };
        return Ok(format!("https://raw.githubusercontent.com/{owner}/{repo}/HEAD/{tail}"));
    }

    if s.starts_with("https://") {
        return if s.ends_with(".md") {
            Ok(s.to_string())
        } else {
            Err("that URL doesn't point at a SKILL.md file".into())
        };
    }
    if s.starts_with("http://") {
        return Err("refusing to install over plain HTTP — anyone on the network could rewrite the instructions in transit".into());
    }
    // Bare `owner/repo[/path]`.
    let parts: Vec<&str> = s.split('/').filter(|p| !p.is_empty()).collect();
    if parts.len() >= 2 && parts.iter().all(|p| p.chars().all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))) {
        let (owner, repo) = (parts[0], parts[1]);
        let tail = if parts.len() > 2 { format!("{}/SKILL.md", parts[2..].join("/")) } else { "SKILL.md".into() };
        return Ok(format!("https://raw.githubusercontent.com/{owner}/{repo}/HEAD/{tail}"));
    }
    Err(format!("didn't understand '{source}' — try `owner/repo`, a GitHub link, or an https URL to a SKILL.md"))
}

/// Fetch and parse a skill. Writes nothing.
pub async fn preview(data_dir: &Path, source: &str) -> Result<Preview, String> {
    let url = resolve_source(source)?;
    // The same egress guard the web tools use: a "skill source" is a URL the user
    // typed, and localhost or a private address is someone probing the machine.
    if crate::core::webguard::is_blocked_url(&url) {
        return Err("that address isn't allowed — skills must come from a public host".into());
    }

    let resp = crate::core::egress::client("skills")
        .get(&url)
        .header("User-Agent", "Superuser")
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| format!("couldn't fetch {url}: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("{url} returned {} — check the path (a skill lives at <dir>/SKILL.md)", resp.status()));
    }
    let body = resp.text().await.map_err(|e| e.to_string())?;
    if body.len() > MAX_BYTES {
        return Err(format!("that SKILL.md is {} KB — too large to review, and skills are meant to be short", body.len() / 1024));
    }

    // The trailing path segment is the conventional skill name (`skills/foo/SKILL.md`).
    let fallback = url
        .trim_end_matches("/SKILL.md")
        .rsplit('/')
        .next()
        .unwrap_or("skill")
        .to_string();
    let skill = parse_skill(&body, &fallback, None)?;
    if !safe_name(&skill.name) {
        return Err(format!("'{}' isn't a usable skill name", skill.name));
    }

    Ok(Preview {
        replaces_existing: data_dir.join("skills").join(&skill.name).join("SKILL.md").exists(),
        name: skill.name,
        description: skill.description,
        instructions: skill.instructions,
        allowed_tools: skill.allowed_tools,
        source: source.trim().to_string(),
        url,
    })
}

/// A skill name has to be a single, ordinary directory name: it becomes a path.
pub(super) fn safe_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name != "."
        && name != ".."
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Write an approved preview to `skills/<name>/SKILL.md`.
///
/// Takes the `Preview` rather than the source string on purpose: installing
/// re-fetches nothing, so what lands on disk is exactly the text the user read.
/// Re-resolving the URL here would leave a window where the remote content could
/// differ from what was approved.
pub fn install(data_dir: &Path, preview: &Preview) -> Result<std::path::PathBuf, String> {
    if !safe_name(&preview.name) {
        return Err(format!("'{}' isn't a usable skill name", preview.name));
    }
    let dir = data_dir.join("skills").join(&preview.name);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join("SKILL.md");
    // Serialized as real YAML, not formatted by hand. A description carrying a
    // colon or a newline would otherwise either break the file or smuggle in extra
    // frontmatter keys — `allowed-tools: run_shell` hidden inside a description is
    // a plausible thing for a hostile skill to try.
    let mut fm = serde_yaml_ng::Mapping::new();
    let put = |m: &mut serde_yaml_ng::Mapping, k: &str, v: serde_yaml_ng::Value| {
        m.insert(serde_yaml_ng::Value::String(k.into()), v);
    };
    put(&mut fm, "name", preview.name.clone().into());
    put(&mut fm, "description", preview.description.clone().into());
    if !preview.allowed_tools.is_empty() {
        put(&mut fm, "allowed-tools", preview.allowed_tools.join(", ").into());
    }
    // Provenance lives in the file so it survives in the user's own editor: months
    // later, "where did this come from" has an answer on disk.
    put(&mut fm, "source", preview.url.clone().into());
    put(&mut fm, "installed", chrono::Local::now().format("%Y-%m-%d").to_string().into());

    let yaml = serde_yaml_ng::to_string(&fm).map_err(|e| e.to_string())?;
    let content = format!("---\n{}---\n\n{}\n", yaml, preview.instructions.trim());
    std::fs::write(&path, content).map_err(|e| e.to_string())?;
    crate::core::audit::log(data_dir, "skill", "user", &preview.name, &format!("installed from {}", preview.url), true);
    Ok(path)
}

/// Remove an installed skill. Built-ins are compiled into the binary and simply
/// have no directory, so this can only ever remove something the user added.
pub fn uninstall(data_dir: &Path, name: &str) -> bool {
    if !safe_name(name) {
        return false;
    }
    let dir = data_dir.join("skills").join(name);
    dir.is_dir() && std::fs::remove_dir_all(&dir).is_ok()
}
