//! Real GitHub integration (personal access token). Fetches the user's profile
//! and recent activity and writes each item into the local Memory Tree as a
//! markdown memory. Idempotent: events are keyed by id so re-syncing dedupes.

use crate::core::memory::{MemoryEngine, MemoryInput, Origin};
use serde_json::Value;

async fn gh(client: &crate::core::egress::Client, token: &str, url: &str) -> Result<Value, String> {
    let resp = client
        .get(url)
        .header("Authorization", format!("Bearer {token}"))
        .header("User-Agent", "Superuser")
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .await
        .map_err(|e| format!("Network error reaching GitHub: {e}"))?;

    match resp.status().as_u16() {
        200 => resp.json().await.map_err(|e| e.to_string()),
        401 => Err("GitHub rejected the token (401). Check it's valid and not expired.".into()),
        403 => Err("GitHub rate limit or insufficient scope (403).".into()),
        s => Err(format!("GitHub returned status {s}.")),
    }
}

/// Validate a token and return the account label (e.g. "@octocat").
pub async fn verify(token: &str) -> Result<String, String> {
    let client = crate::core::egress::client("github");
    let me = gh(&client, token, "https://api.github.com/user").await?;
    me["login"].as_str().map(|l| format!("@{l}")).ok_or_else(|| "GitHub did not return a username.".into())
}

/// Sync recent GitHub activity into the vault.
pub async fn sync(token: &str, memory: &MemoryEngine) -> Result<super::SyncOutcome, String> {
    let client = crate::core::egress::client("github");

    let me = gh(&client, token, "https://api.github.com/user").await?;
    let login = me["login"].as_str().ok_or("Could not read your GitHub profile.")?.to_string();

    let mut out = super::SyncOutcome::default();
    let mut write = |input: MemoryInput| {
        let fname = input.filename.clone().unwrap_or_default();
        let existed = !fname.is_empty() && memory.vault_path.join(&fname).exists();
        let title = input.title.clone();
        if memory.save(&input).is_ok() && !existed {
            out.created += 1;
            out.titles.push(title);
        }
    };

    // Profile summary memory
    write(MemoryInput {
        filename: Some("github-profile.md".into()),
        title: format!("GitHub profile: {login}"),
        category: "GitHub".into(),
        tags: vec!["github".into(), "profile".into()],
        summary: format!("{} · {} public repos · {} followers", me["name"].as_str().unwrap_or(&login), me["public_repos"].as_i64().unwrap_or(0), me["followers"].as_i64().unwrap_or(0)),
        body: format!("GitHub user **{login}**.\n\n- Public repos: {}\n- Followers: {}\n- Bio: {}", me["public_repos"].as_i64().unwrap_or(0), me["followers"].as_i64().unwrap_or(0), me["bio"].as_str().unwrap_or("—")),
        origin: Origin::External,
        key: None,
        supersedes: None,
    });

    // Recent activity (private events included when authenticated as yourself)
    let events = gh(&client, token, &format!("https://api.github.com/users/{login}/events?per_page=40")).await?;
    if let Some(arr) = events.as_array() {
        for ev in arr {
            if let Some(input) = event_to_memory(ev) {
                write(input);
            }
        }
    }

    // Recently pushed repos
    let repos = gh(&client, token, "https://api.github.com/user/repos?sort=pushed&per_page=12&affiliation=owner,collaborator").await?;
    if let Some(arr) = repos.as_array() {
        for r in arr {
            let name = r["full_name"].as_str().unwrap_or("");
            if name.is_empty() { continue; }
            write(MemoryInput {
                filename: Some(format!("github-repo-{}.md", name.replace('/', "-"))),
                title: format!("Repo: {name}"),
                category: "GitHub".into(),
                tags: vec!["github".into(), "repo".into()],
                summary: r["description"].as_str().unwrap_or("").to_string(),
                body: format!("**{name}**\n\n{}\n\n- Language: {}\n- Stars: {}\n- Last push: {}", r["description"].as_str().unwrap_or("No description."), r["language"].as_str().unwrap_or("—"), r["stargazers_count"].as_i64().unwrap_or(0), r["pushed_at"].as_str().unwrap_or("")),
                origin: Origin::External,
                key: None,
                supersedes: None,
            });
        }
    }

    Ok(out)
}

/// Map a GitHub event JSON into a memory (or None to skip).
fn event_to_memory(ev: &Value) -> Option<MemoryInput> {
    let id = ev["id"].as_str()?;
    let typ = ev["type"].as_str()?;
    let repo = ev["repo"]["name"].as_str().unwrap_or("");
    let p = &ev["payload"];

    let (title, body) = match typ {
        "PushEvent" => {
            let commits = p["commits"].as_array();
            let n = commits.map(|c| c.len()).unwrap_or(0);
            let msgs = commits.map(|c| c.iter().map(|x| format!("- {}", x["message"].as_str().unwrap_or("").lines().next().unwrap_or(""))).collect::<Vec<_>>().join("\n")).unwrap_or_default();
            (format!("Pushed {n} commit(s) to {repo}"), msgs)
        }
        "PullRequestEvent" => {
            let action = p["action"].as_str().unwrap_or("updated");
            let num = p["number"].as_i64().or_else(|| p["pull_request"]["number"].as_i64()).unwrap_or(0);
            let t = p["pull_request"]["title"].as_str().unwrap_or("");
            (format!("{action} PR #{num} in {repo}"), format!("**{t}**\n\n{}", p["pull_request"]["body"].as_str().unwrap_or("")))
        }
        "IssuesEvent" => {
            let action = p["action"].as_str().unwrap_or("updated");
            let num = p["issue"]["number"].as_i64().unwrap_or(0);
            let t = p["issue"]["title"].as_str().unwrap_or("");
            (format!("{action} issue #{num} in {repo}"), format!("**{t}**\n\n{}", p["issue"]["body"].as_str().unwrap_or("")))
        }
        "IssueCommentEvent" => {
            let num = p["issue"]["number"].as_i64().unwrap_or(0);
            (format!("Commented on #{num} in {repo}"), p["comment"]["body"].as_str().unwrap_or("").to_string())
        }
        "WatchEvent" => (format!("Starred {repo}"), String::new()),
        "ForkEvent" => (format!("Forked {repo}"), String::new()),
        "CreateEvent" => (format!("Created {} in {repo}", p["ref_type"].as_str().unwrap_or("ref")), String::new()),
        "ReleaseEvent" => (format!("Released {} in {repo}", p["release"]["tag_name"].as_str().unwrap_or("")), p["release"]["name"].as_str().unwrap_or("").to_string()),
        _ => return None, // skip noisy/uncommon event types
    };

    let mut tags = vec!["github".into(), "activity".into()];
    if let Some(short) = repo.split('/').next_back() { tags.push(short.to_string()); }

    Some(MemoryInput {
        filename: Some(format!("github-{id}.md")),
        title,
        category: "GitHub".into(),
        tags,
        summary: format!("{} · {}", typ, ev["created_at"].as_str().unwrap_or("")),
        body: if body.trim().is_empty() { format!("GitHub {typ} on {repo}.") } else { body },
        origin: Origin::External,
        key: None,
        supersedes: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_push_event() {
        let ev: Value = serde_json::json!({
            "id": "123", "type": "PushEvent", "repo": {"name": "me/superuser"},
            "created_at": "2026-06-10T10:00:00Z",
            "payload": {"commits": [{"message": "fix: thing"}, {"message": "feat: stuff"}]}
        });
        let m = event_to_memory(&ev).unwrap();
        assert_eq!(m.filename.as_deref(), Some("github-123.md"));
        assert!(m.title.contains("Pushed 2 commit"));
        assert!(m.body.contains("fix: thing"));
        assert_eq!(m.category, "GitHub");
    }

    #[test]
    fn skips_unknown_event() {
        let ev: Value = serde_json::json!({ "id": "9", "type": "GollumEvent", "repo": {"name": "a/b"}, "payload": {} });
        assert!(event_to_memory(&ev).is_none());
    }
}
