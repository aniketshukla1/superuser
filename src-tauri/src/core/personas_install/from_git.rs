use super::*;

/// Markdown files directly inside a folder — and inside its `personas/`
/// subdirectory, which is how a repo that also contains other things lays them out.
///
/// One level only, on purpose: recursing means a folder someone points at by
/// mistake (a home directory, a checkout of something else) turns into a long walk
/// over their disk.
pub(super) fn from_dir(dir: &str) -> Result<Vec<Candidate>, String> {
    let root = PathBuf::from(shellexpand(dir.trim()));
    if !root.is_dir() {
        return Err(format!("{} isn't a folder on this machine", root.display()));
    }
    let mut out = Vec::new();
    for base in [root.clone(), root.join("personas")] {
        let Ok(rd) = std::fs::read_dir(&base) else { continue };
        for entry in rd.flatten() {
            let p = entry.path();
            if p.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            // README.md is documentation about the bundle, not a member of it, and
            // it has no frontmatter — parsing it would fail the whole install.
            if p.file_stem().and_then(|s| s.to_str()).map(|s| s.eq_ignore_ascii_case("readme")).unwrap_or(false) {
                continue;
            }
            let Ok(meta) = entry.metadata() else { continue };
            if meta.len() as usize > MAX_BYTES {
                return Err(format!("{} is too large to be a persona manifest", p.display()));
            }
            let Ok(text) = std::fs::read_to_string(&p) else { continue };
            out.push(Candidate { text, origin: p.to_string_lossy().into_owned() });
        }
        if !out.is_empty() {
            break; // the root had manifests; don't also sweep personas/
        }
    }
    Ok(out)
}

/// `~` only. Not a shell — expanding anything else here would mean running one.
pub(super) fn shellexpand(p: &str) -> String {
    match p.strip_prefix("~/") {
        Some(rest) => std::env::var("HOME").map(|h| format!("{h}/{rest}")).unwrap_or_else(|_| p.to_string()),
        None => p.to_string(),
    }
}

/// `owner/repo` out of the shapes people paste into the address field.
pub(super) fn repo_of(url: &str) -> Result<(String, String, Option<String>), String> {
    let s = url.trim().trim_end_matches('/').trim_end_matches(".git");
    if s.is_empty() {
        return Err("paste a GitHub repository — `owner/repo` or its web address".into());
    }
    if s.starts_with("http://") {
        return Err("refusing to install over plain HTTP — anyone on the network could rewrite the instructions in transit".into());
    }
    let rest = s
        .strip_prefix("https://github.com/")
        .or_else(|| s.strip_prefix("git@github.com:"))
        .unwrap_or(s);
    let parts: Vec<&str> = rest.split('/').filter(|p| !p.is_empty()).collect();
    if parts.len() < 2 || !parts.iter().all(|p| p.chars().all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))) {
        return Err(format!("didn't understand '{url}' — try `owner/repo` or a github.com link"));
    }
    // `.../tree/<ref>/<path>` is what the address bar gives you when you are looking
    // at a subfolder, and a subfolder is exactly how a repo ships several bundles.
    let sub = if parts.len() > 3 && (parts[2] == "tree" || parts[2] == "blob") {
        Some(parts[4..].join("/")).filter(|p: &String| !p.is_empty())
    } else if parts.len() > 2 {
        Some(parts[2..].join("/"))
    } else {
        None
    };
    Ok((parts[0].to_string(), parts[1].to_string(), sub))
}

/// Markdown files in a GitHub repo, over the contents API.
///
/// HTTPS reads only — see the module note on why this is not a `git clone`.
pub(super) async fn from_git(url: &str) -> Result<Vec<Candidate>, String> {
    let (owner, repo, sub) = repo_of(url)?;
    let client = crate::core::egress::client("personas");

    // Try the named subfolder, then `personas/`, then the root. The first that has
    // manifests wins, so a repo with a README at the root and bundles underneath
    // does the right thing without the user having to say so.
    let mut tried: Vec<String> = Vec::new();
    for path in [sub.clone(), Some("personas".to_string()), Some(String::new())].into_iter().flatten() {
        let api = format!(
            "https://api.github.com/repos/{owner}/{repo}/contents/{}",
            path.trim_matches('/')
        );
        if crate::core::webguard::is_blocked_url(&api) {
            return Err("that address isn't allowed — assistants must come from a public host".into());
        }
        tried.push(if path.is_empty() { "the repository root".into() } else { path.clone() });
        let resp = client
            .get(&api)
            .header("User-Agent", "Superuser")
            .header("Accept", "application/vnd.github+json")
            .timeout(std::time::Duration::from_secs(20))
            .send()
            .await
            .map_err(|e| format!("couldn't reach {api}: {e}"))?;
        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            continue;
        }
        if !resp.status().is_success() {
            return Err(format!("github answered {} for {owner}/{repo}", resp.status()));
        }
        let body: serde_json::Value = resp.json().await.map_err(|e| format!("github sent something unreadable: {e}"))?;
        let Some(entries) = body.as_array() else { continue };

        let mut out = Vec::new();
        for e in entries {
            if e["type"].as_str() != Some("file") {
                continue;
            }
            let name = e["name"].as_str().unwrap_or("");
            if !name.ends_with(".md") || name.eq_ignore_ascii_case("readme.md") {
                continue;
            }
            if e["size"].as_u64().unwrap_or(0) as usize > MAX_BYTES {
                return Err(format!("{name} is too large to be a persona manifest"));
            }
            // `download_url` is github's own raw link for this exact blob. Vetted
            // like any other URL rather than trusted because of where it came from:
            // it arrives inside a response body, which is not a place instructions
            // or addresses get to come from unchecked.
            let Some(raw) = e["download_url"].as_str() else { continue };
            if crate::core::webguard::is_blocked_url(raw) {
                return Err(format!("{name} points somewhere that isn't allowed"));
            }
            let r = client
                .get(raw)
                .header("User-Agent", "Superuser")
                .timeout(std::time::Duration::from_secs(20))
                .send()
                .await
                .map_err(|err| format!("couldn't fetch {name}: {err}"))?;
            if !r.status().is_success() {
                return Err(format!("{name} returned {}", r.status()));
            }
            let text = r.text().await.map_err(|err| err.to_string())?;
            if text.len() > MAX_BYTES {
                return Err(format!("{name} is too large to review"));
            }
            out.push(Candidate { text, origin: raw.to_string() });
        }
        if !out.is_empty() {
            return Ok(out);
        }
    }
    Err(format!("found no persona manifests in {owner}/{repo} (looked in {})", tried.join(", ")))
}
