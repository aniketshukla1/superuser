use super::*;

/// A scanner this app knows how to talk about.
pub struct Tool {
    pub name: &'static str,
    /// The check this tool performs, in the words the Coverage note will use.
    pub check: &'static str,
    /// What is actually lost when it is missing. Not a shrug — the note quotes
    /// this, and a user deciding whether to install something deserves to know
    /// what stays uncovered if they don't.
    pub fallback: &'static str,
    /// The exact command that installs it. Shown on the approval card verbatim,
    /// so what the user reads is what runs.
    pub install: &'static str,
}

/// The closed catalogue.
pub const CATALOG: &[Tool] = &[
    Tool {
        name: "gitleaks",
        check: "Secret sweep (working tree + full git history)",
        fallback: "history can be swept by hand with `git log -p`, which is slower and \
                   misses secrets in deleted files unless those are read too",
        install: "brew install gitleaks",
    },
    Tool {
        name: "semgrep",
        check: "Static analysis for code vulnerabilities",
        fallback: "routes, handlers, auth code, string-built queries and deserialization \
                   can be read by hand — narrower coverage than a full rule sweep",
        install: "brew install semgrep",
    },
    Tool {
        name: "trivy",
        check: "Vulnerability scan of images, filesystems and IaC",
        fallback: "no equivalent — lockfile advisories can be checked one at a time",
        install: "brew install trivy",
    },
    Tool {
        name: "osv-scanner",
        check: "Dependency vulnerabilities from the OSV database",
        fallback: "`npm audit` / `pip-audit` cover their own ecosystems only",
        install: "brew install osv-scanner",
    },
    Tool {
        name: "checkov",
        check: "Terraform and cloud-configuration policy checks",
        fallback: "Terraform can be read by hand for public buckets, open security \
                   groups and unencrypted volumes",
        install: "brew install checkov",
    },
    Tool {
        name: "pip-audit",
        check: "Python dependency advisories",
        fallback: "no equivalent for Python packages",
        install: "brew install pip-audit",
    },
];

pub fn find(name: &str) -> Option<&'static Tool> {
    let n = name.trim().to_lowercase();
    CATALOG.iter().find(|t| t.name == n)
}

/// The catalogue as one line per tool, for a tool description or an error.
pub fn names() -> String {
    CATALOG.iter().map(|t| t.name).collect::<Vec<_>>().join(", ")
}

/// Is it on this machine, and at what version?
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Presence {
    Present { version: String },
    Missing,
}

/// Ask the machine, through a login shell so Homebrew's paths are on PATH.
///
/// `name` is always a catalogue entry — [`find`] gates every caller — so nothing
/// user- or model-supplied reaches the shell here.
pub fn probe(tool: &Tool) -> Presence {
    let out = Command::new("sh")
        .arg("-lc")
        .arg(format!("command -v {} >/dev/null 2>&1 && {} --version 2>&1 | head -1", tool.name, tool.name))
        .stdin(Stdio::null())
        .output();
    match out {
        Ok(o) if o.status.success() => {
            let text = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if text.is_empty() {
                Presence::Missing
            } else {
                Presence::Present { version: text.chars().take(80).collect() }
            }
        }
        _ => Presence::Missing,
    }
}

/// What happened to one check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Status {
    /// It ran, under a named tool at a named version.
    Ran { tool: String, version: String },
    /// It ran, but narrower than it should have — a manual equivalent, a partial
    /// pass. The note says so and says why.
    Degraded { reason: String },
    /// It did not run at all. This is the entry the whole module exists to keep.
    Skipped { reason: String },
}

/// One line of the Coverage note.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub check: String,
    #[serde(flatten)]
    pub status: Status,
}

/// Record what happened to a check, replacing any earlier entry for the same one.
///
/// Replacement rather than append is the point: `check_tool` writes a `Skipped`
/// the moment it finds a tool absent, and the model amends that to `Degraded`
/// when it does the work another way. Two entries for one check would read as two
/// checks.
pub fn record(ledger: &mut Vec<Entry>, check: &str, status: Status) {
    let check = check.trim();
    if check.is_empty() {
        return;
    }
    if let Some(existing) = ledger.iter_mut().find(|e| e.check == check) {
        existing.status = status;
        return;
    }
    ledger.push(Entry { check: check.to_string(), status });
}

/// Render the note. Ordered worst-first: what did not happen is the part a reader
/// most needs and is least likely to go looking for.
pub fn render(ledger: &[Entry]) -> String {
    if ledger.is_empty() {
        return "**Coverage** — nothing to report: no checks were registered for this run.".into();
    }
    let rank = |e: &Entry| match e.status {
        Status::Skipped { .. } => 0,
        Status::Degraded { .. } => 1,
        Status::Ran { .. } => 2,
    };
    let mut rows: Vec<&Entry> = ledger.iter().collect();
    rows.sort_by_key(|e| rank(e));

    let mut out = String::from("**Coverage**\n\n");
    for e in rows {
        match &e.status {
            Status::Ran { tool, version } => {
                out.push_str(&format!("- ✅ {} — ran under `{tool}` ({version})\n", e.check));
            }
            Status::Degraded { reason } => {
                out.push_str(&format!("- ⚠️ {} — degraded: {reason}\n", e.check));
            }
            Status::Skipped { reason } => {
                out.push_str(&format!("- ❌ {} — NOT RUN: {reason}\n", e.check));
            }
        }
    }
    let skipped = ledger.iter().filter(|e| matches!(e.status, Status::Skipped { .. })).count();
    if skipped > 0 {
        out.push_str(&format!(
            "\n{skipped} check{} did not run. Nothing above says those areas are clean — only that they were not looked at.\n",
            if skipped == 1 { "" } else { "s" }
        ));
    }
    out
}
