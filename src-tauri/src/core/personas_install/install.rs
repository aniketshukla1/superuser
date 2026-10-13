use super::*;

/// Cap on one manifest. Real ones are a few KB; anything near this is either a
/// mistake or an attempt to bury instructions past where a human will read.
pub(super) const MAX_BYTES: usize = 256 * 1024;

/// Cap on manifests taken from one source. A repo offering forty assistants is not
/// a thing anyone reviews; it is a way to get one past the reader.
pub(super) const MAX_PERSONAS: usize = 12;

/// What the user is asked to look at after an install — one card per assistant,
/// answering "what did I just take on".
///
/// `risk` is derived from the capability list rather than declared, because a
/// manifest that got to describe its own risk would describe it as none.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Installed {
    pub id: String,
    pub name: String,
    pub description: String,
    pub tools: Vec<String>,
    pub risk: Vec<crate::core::agent::RiskClass>,
    pub connectors: bool,
    pub mcp: Vec<String>,
    pub messaging: bool,
    pub recommended_mode: String,
    /// The manifest's own claim that a small model judges its work badly. Replaces
    /// the `recommended_models` list this card used to carry, which named model ids
    /// nothing declared and no local-first user could act on.
    pub needs_capable_model: bool,
    pub source: Option<String>,
    pub builtin: bool,
}

impl Installed {
    pub(crate) fn from(m: &PersonaManifest) -> Self {
        Self {
            id: m.id.clone(),
            name: m.name.clone(),
            description: if m.description.trim().is_empty() { m.tagline.clone() } else { m.description.clone() },
            risk: crate::core::agent::risk_summary(&m.tools),
            tools: m.tools.clone(),
            connectors: m.connectors,
            mcp: m.mcp.clone(),
            messaging: m.messaging,
            recommended_mode: m.default_permission_mode.clone(),
            needs_capable_model: m.needs_capable_model,
            source: m.source.clone(),
            builtin: m.builtin,
        }
    }
}

/// Where the manifests come from.
pub enum Source {
    /// A folder on this machine the user pointed at.
    Dir(String),
    /// A GitHub repository, as pasted into the address field.
    Git(String),
}

/// One manifest on its way in: its text and where it came from, before anything
/// has been parsed or written.
pub(super) struct Candidate {
    pub(super) text: String,
    /// For the error message and for the `source:` recorded on disk.
    pub(super) origin: String,
}

/// Install every persona a source offers. Returns a card per assistant installed.
///
/// Writes manifests and nothing else. The caller re-reads the registry afterwards;
/// each one arrives disabled.
pub async fn install(data_dir: &Path, source: Source) -> Result<Vec<Installed>, String> {
    let mut candidates = match source {
        Source::Dir(d) => from_dir(&d)?,
        Source::Git(u) => from_git(&u).await?,
    };
    if candidates.is_empty() {
        return Err("no persona manifests there — a persona is a Markdown file with `---` frontmatter naming it".into());
    }
    if candidates.len() > MAX_PERSONAS {
        return Err(format!(
            "that source offers {} assistants — more than anyone reviews in one go. Install them a few at a time from subfolders.",
            candidates.len()
        ));
    }
    candidates.sort_by(|a, b| a.origin.cmp(&b.origin));

    // A Markdown file with no frontmatter is documentation, not a broken manifest.
    // This matters because a source is often a whole repo: CONTRIBUTING.md sitting
    // beside the bundle should not fail the install, while a manifest that opens
    // `---` and then goes wrong absolutely should — that one was TRYING to be an
    // assistant, and skipping it quietly is how you install two of three and are
    // told it all worked.
    candidates.retain(|c| c.text.trim_start().starts_with("---"));
    if candidates.is_empty() {
        return Err("no persona manifests there — a persona is a Markdown file with `---` frontmatter naming it".into());
    }

    // Parse everything BEFORE writing anything. A source whose third manifest is
    // malformed should not leave the first two installed and the user guessing
    // which half landed.
    let registry = PersonaRegistry::load(data_dir);
    let mut parsed: Vec<(PersonaManifest, String)> = Vec::new();
    for c in &candidates {
        let stem = Path::new(&c.origin).file_stem().and_then(|s| s.to_str()).unwrap_or("persona");
        let m = parse_manifest(&c.text, Some(stem), false, Some(c.origin.clone()))
            .map_err(|e| format!("{}: {e}", c.origin))?;
        if !safe_id(&m.id) {
            return Err(format!("{}: '{}' isn't a usable assistant id", c.origin, m.id));
        }
        // A built-in's manifest is compiled into the binary, so a file of the same
        // name would not replace it — it would sit on disk being ignored while the
        // user believed they had installed something. Refuse, and say why.
        if registry.manifest(&m.id).map(|e| e.builtin).unwrap_or(false) {
            return Err(format!("'{}' is the id of a built-in assistant — rename it in the manifest", m.id));
        }
        if parsed.iter().any(|(p, _)| p.id == m.id) {
            return Err(format!("that source has two assistants both called '{}'", m.id));
        }
        parsed.push((m, c.text.clone()));
    }

    let pdir = data_dir.join("personas");
    std::fs::create_dir_all(&pdir).map_err(|e| format!("couldn't create the personas folder: {e}"))?;
    let mut out = Vec::new();
    for (m, text) in &parsed {
        let path = pdir.join(format!("{}.md", m.id));
        std::fs::write(&path, text).map_err(|e| format!("couldn't write {}: {e}", path.display()))?;
        crate::core::audit::log(
            data_dir,
            "persona",
            "user",
            &m.id,
            &format!("installed from {}", m.source.as_deref().unwrap_or("a local folder")),
            true,
        );
        out.push(Installed::from(m));
    }

    // Re-read so the cards carry what the registry will actually serve rather than
    // what we parsed in memory a moment ago — `builtin: false`, and any field the
    // registry normalises on load.
    //
    // `source` is the exception and has to be put back. On disk it is the managed
    // copy's own path, which answers "where does this file live" when the question
    // the card is asking is "where did this come from" — and a card that says
    // `.../Application Support/.../acme-ops.md` under "From" has told the user
    // nothing they could act on.
    let after = PersonaRegistry::load(data_dir);
    Ok(out
        .into_iter()
        .map(|c| match after.manifest(&c.id) {
            Some(m) => Installed { source: c.source.clone(), ..Installed::from(m) },
            None => c,
        })
        .collect())
}

/// An id becomes a filename, so it has to be an ordinary one.
pub(super) fn safe_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id != "."
        && id != ".."
        && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}
