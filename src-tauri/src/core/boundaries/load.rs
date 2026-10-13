use super::*;

pub(super) fn compile(b: Boundaries, problem: Option<String>) -> Loaded {
    Loaded {
        compiled_deny: b.deny.iter().map(|r| rules::parse(&r.when)).collect(),
        compiled_allow: b.allow.iter().map(|a| rules::parse(a)).collect(),
        boundaries: b,
        problem,
    }
}

/// The compiled boundaries, re-read only when the file changes. A tool call checks
/// these every time, and parsing every rule per call is a cost that buys nothing.
pub(super) type Cache = Option<(PathBuf, Option<std::time::SystemTime>, u64, Arc<Loaded>)>;

pub(super) static CACHE: Mutex<Cache> = Mutex::new(None);

/// The boundaries in force.
///
/// A missing file is the default: nothing refused. A file that exists and cannot be
/// read is not — silently falling back to the default would quietly lift every rule
/// the user wrote, which is the one outcome this file must never produce. So it
/// loads as a single broken rule, which refuses everything with a sentence that
/// names the file, until it is fixed or reset on the Boundaries page.
pub fn load(data_dir: &Path) -> Arc<Loaded> {
    let p = path(data_dir);
    let meta = std::fs::metadata(&p).ok();
    let stamp = meta.as_ref().and_then(|m| m.modified().ok());
    let len = meta.as_ref().map(|m| m.len()).unwrap_or(0);
    if let Ok(cache) = CACHE.lock() {
        if let Some((cp, cs, cl, loaded)) = cache.as_ref() {
            if *cp == p && *cs == stamp && *cl == len {
                return loaded.clone();
            }
        }
    }
    let loaded = Arc::new(match std::fs::read_to_string(&p) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => compile(Boundaries::default(), None),
        Err(e) => unreadable(format!("{FILE} couldn't be read ({e})")),
        Ok(text) => match serde_json::from_str::<Boundaries>(&text) {
            Ok(b) => compile(b, None),
            Err(e) => unreadable(format!("{FILE} isn't valid ({e})")),
        },
    });
    if let Ok(mut cache) = CACHE.lock() {
        *cache = Some((p, stamp, len, loaded.clone()));
    }
    loaded
}

pub(super) fn unreadable(problem: String) -> Loaded {
    compile(
        Boundaries {
            mode: Mode::Enforce,
            deny: vec![Rule { name: "Your boundaries file can't be read".into(), when: "true".into(), preset: String::new() }],
            allow: vec![],
        },
        Some(problem),
    )
}

/// Check every rule, then write the file. Nothing invalid is ever saved: a broken
/// deny rule would refuse everything it might cover, and the person saving it is
/// the one who should hear about that, not their next task.
pub fn save(data_dir: &Path, b: &Boundaries) -> Result<(), Vec<RuleProblem>> {
    let problems = check(b);
    if !problems.is_empty() {
        return Err(problems);
    }
    let json = serde_json::to_string_pretty(b).map_err(|e| vec![RuleProblem::whole(e.to_string())])?;
    crate::core::store::write_atomic(&path(data_dir), &json).map_err(|e| vec![RuleProblem::whole(e.to_string())])?;
    let mut e = Entry::event("policy.changed");
    e.actor = "user".into();
    e.detail = summary(b);
    crate::core::audit::append(data_dir, e);
    Ok(())
}

/// Throw the saved boundaries away and go back to the default — the way out of an
/// unreadable file that doesn't require a text editor.
pub fn reset(data_dir: &Path) -> bool {
    let removed = std::fs::remove_file(path(data_dir)).is_ok();
    let mut e = Entry::event("policy.changed");
    e.actor = "user".into();
    e.detail = "Boundaries reset: nothing is refused.".into();
    crate::core::audit::append(data_dir, e);
    removed
}

/// The boundaries in a sentence, for the trail.
pub fn summary(b: &Boundaries) -> String {
    let names: Vec<String> = b
        .deny
        .iter()
        .map(|r| if r.name.is_empty() { format!("`{}`", r.when.chars().take(60).collect::<String>()) } else { r.name.clone() })
        .collect();
    format!(
        "{} · {} · allow: {}",
        match b.mode {
            Mode::Enforce => "refusing",
            Mode::Record => "recording only",
        },
        if names.is_empty() { "no rules".to_string() } else { names.join("; ") },
        if b.allow.is_empty() { "nothing".to_string() } else { b.allow.join(" | ") }
    )
}

/// Written once at startup: what the boundaries in force were, so a trail read later
/// can say what was being enforced at the time.
pub fn record_loaded(data_dir: &Path) {
    let loaded = load(data_dir);
    let mut e = Entry::event("policy.loaded");
    e.actor = "app".into();
    e.initiator = Some(Initiator::new("startup", ""));
    e.ok = loaded.problem.is_none();
    e.detail = match &loaded.problem {
        Some(p) => format!("{p} — every action is refused until it is fixed or reset"),
        None => summary(&loaded.boundaries),
    };
    crate::core::audit::append(data_dir, e);
}

/// What is wrong with one rule, for the page to show beside it.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct RuleProblem {
    /// `deny` or `allow`, or empty for the whole file.
    pub list: String,
    pub index: usize,
    pub message: String,
}

impl RuleProblem {
    pub(super) fn whole(message: String) -> Self {
        RuleProblem { list: String::new(), index: 0, message }
    }
}

/// The facts a rule reads, with every field present.
///
/// Every field is bound, empty when it does not apply, so a rule about commands
/// answers `false` about a file write instead of failing to find `command` — which on
/// a deny list would refuse every file write. The same function builds the context
/// live and in [`replay`], so a rule cannot behave one way in Test and another way
/// for real.
pub fn context(s: &Subject, persona: &str, session: &str, initiator: &Initiator) -> Value {
    let (name, extension) = file_parts(&s.file);
    let host = host_of(&s.page);
    json!({
        "intent": s.intent,
        "tool": { "name": s.tool, "risk": s.risk },
        "command": s.command,
        "file": { "path": s.file, "name": name, "extension": extension },
        "page": { "url": s.page, "host": host },
        "target": s.target,
        "mcp": { "server": s.mcp_server, "tool": s.mcp_tool, "effect": s.mcp_effect },
        "persona": { "id": persona },
        "bot": { "id": persona },
        "actor": { "id": "user" },
        "session": { "id": session },
        "initiator": { "kind": initiator.kind, "id": initiator.id },
        "unattended": initiator.unwatched(),
        "tainted": s.tainted,
        "chars": s.chars.unwrap_or(0),
        "key": s.key,
        "element": { "ref": s.element_ref, "role": s.element_role, "name": s.element_name, "type": s.element_type },
    })
}

/// A file's name and extension, the way a rule wants them: lower-case extension
/// without the dot, and none at all for a dotfile — `.env` has no extension, it IS
/// the name, which is what makes `file.name == ".env"` the rule that works.
pub fn file_parts(path: &str) -> (String, String) {
    let name = path.rsplit(['/', '\\']).next().unwrap_or("").to_string();
    let extension = match name.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() => ext.to_ascii_lowercase(),
        _ => String::new(),
    };
    (name, extension)
}

pub fn host_of(url: &str) -> String {
    let Some((_, rest)) = url.split_once("://") else { return String::new() };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    let authority = authority.rsplit('@').next().unwrap_or(authority);
    let host = if let Some(v6) = authority.strip_prefix('[') {
        v6.split(']').next().unwrap_or("")
    } else {
        authority.rsplit_once(':').map(|(h, _)| h).unwrap_or(authority)
    };
    host.trim_end_matches('.').to_ascii_lowercase()
}
