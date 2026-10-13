//! Source control

use super::*;

/// Run `git -C root <args>`; returns stdout on success, the first stderr line on
/// failure. GIT_TERMINAL_PROMPT=0 makes auth failures error fast instead of hang.
pub(super) fn run_git(root: &str, args: &[&str]) -> Result<String, String> {
    let out = Command::new("git").arg("-C").arg(root).args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output().map_err(|e| format!("git not available: {e}"))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).to_string())
    } else {
        let e = String::from_utf8_lossy(&out.stderr);
        Err(e.trim().lines().next().unwrap_or("git error").to_string())
    }
}

#[derive(Serialize)]
pub struct Commit { pub hash: String, pub short: String, pub author: String, pub date: String, pub subject: String }

/// Old/new content for a changed file, for the diff viewer. For a staged change
/// this compares HEAD→index; for an unstaged change, index→working tree. Missing
/// sides (new/untracked or deleted files) come back as empty strings.
pub fn git_diff(root: &str, path: &str, staged: bool) -> (String, String) {
    let show = |spec: String| run_git(root, &["show", &spec]).unwrap_or_default();
    if staged {
        (show(format!("HEAD:{path}")), show(format!(":{path}")))
    } else {
        let old = show(format!(":{path}"));
        let work = std::fs::read_to_string(Path::new(root).join(path)).unwrap_or_default();
        (old, work)
    }
}

/// Recent commit history (newest first).
pub fn git_log(root: &str) -> Vec<Commit> {
    let out = run_git(root, &["log", "--pretty=format:%H%x1f%h%x1f%an%x1f%ad%x1f%s", "--date=short", "-n", "80"]).unwrap_or_default();
    out.lines().filter_map(|l| {
        let mut p = l.split('\u{1f}');
        Some(Commit {
            hash: p.next()?.to_string(),
            short: p.next()?.to_string(),
            author: p.next()?.to_string(),
            date: p.next()?.to_string(),
            subject: p.next().unwrap_or("").to_string(),
        })
    }).collect()
}

/// Current git branch + number of dirty files for the workspace.
pub fn git(root: &str) -> serde_json::Value {
    let branch = Command::new("git").args(["-C", root, "rev-parse", "--abbrev-ref", "HEAD"]).output();
    let (is_git, branch) = match branch {
        Ok(o) if o.status.success() => (true, String::from_utf8_lossy(&o.stdout).trim().to_string()),
        _ => (false, String::new()),
    };
    let dirty = Command::new("git").args(["-C", root, "status", "--porcelain"]).output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).lines().filter(|l| !l.trim().is_empty()).count())
        .unwrap_or(0);
    serde_json::json!({ "is_git": is_git, "branch": branch, "dirty": dirty })
}

/// Grep the workspace (literal, case-insensitive) for the agent. Returns up to
/// `cap` "relative/path:line: text" hits, skipping heavy/binary files.
pub fn grep(root: &str, query: &str, cap: usize) -> String {
    let root_c = match Path::new(root).canonicalize() { Ok(p) => p, Err(_) => return "workspace error".into() };
    let q = query.to_lowercase();
    let mut hits = Vec::new();
    let mut stack = vec![root_c.clone()];
    while let Some(dir) = stack.pop() {
        let rd = match std::fs::read_dir(&dir) { Ok(r) => r, Err(_) => continue };
        for e in rd.flatten() {
            let p = e.path();
            let nm = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if nm.starts_with('.') || SKIP.contains(&nm) { continue; }
            if p.is_dir() { stack.push(p); continue; }
            if let Ok(bytes) = std::fs::read(&p) {
                if bytes.len() > MAX_READ || bytes.iter().take(1024).any(|&b| b == 0) { continue; }
                let text = String::from_utf8_lossy(&bytes);
                let rel = p.strip_prefix(&root_c).unwrap_or(&p).to_string_lossy();
                for (i, line) in text.lines().enumerate() {
                    if line.to_lowercase().contains(&q) {
                        hits.push(format!("{}:{}: {}", rel, i + 1, line.trim().chars().take(160).collect::<String>()));
                        if hits.len() >= cap { return hits.join("\n"); }
                    }
                }
            }
        }
    }
    if hits.is_empty() { "no matches".into() } else { hits.join("\n") }
}
