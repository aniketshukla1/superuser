use super::super::*;

/// Parse + strictly validate a persona manifest. On any problem returns an error
/// string (a third-party persona must fail loudly, not load broken).
pub fn parse_manifest(
    text: &str,
    fallback_id: Option<&str>,
    builtin: bool,
    source: Option<String>,
) -> Result<PersonaManifest, String> {
    let (raw, body) = split_frontmatter(text)?;

    let persona_id = match raw.id.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(explicit) => {
            if !id_re().is_match(explicit) {
                return Err(format!(
                    "persona id '{explicit}' isn't allowed: use up to 64 lowercase letters, digits, '-' and '_', \
                     beginning with a letter or digit"
                ));
            }
            explicit.to_string()
        }
        None => {
            let s = slugify(fallback_id.unwrap_or(""));
            if s.is_empty() {
                return Err("manifest has no `id`, and no filename to make one from".into());
            }
            s
        }
    };

    if body.trim().is_empty() {
        return Err(format!("persona '{persona_id}' has no body (the system prompt)"));
    }

    let family = raw.family.as_deref().unwrap_or("knowledge").trim().to_lowercase();
    if !VALID_FAMILIES.contains(&family.as_str()) {
        return Err(format!("persona '{persona_id}': family must be one of {VALID_FAMILIES:?}"));
    }
    // The workspace enum collapsed into family: code → an explicit git dir, knowledge →
    // a transparent deliverable scratch. The manifest `workspace:` key is still accepted
    // (tolerated) but no longer drives behavior.
    let workspace = if family == "code" { "git" } else { "deliverable" }.to_string();

    let mode = raw.default_permission_mode.as_deref().unwrap_or("interactive").trim().to_lowercase();
    if !VALID_MODES.contains(&mode.as_str()) {
        return Err(format!("persona '{persona_id}': default_permission_mode must be one of {VALID_MODES:?}"));
    }

    let tools = clean_list(raw.tools);
    if let Some(unknown) = tools.iter().find(|t| !KNOWN_TOOLS.contains(&t.as_str())) {
        return Err(format!(
            "persona '{persona_id}' references unknown tool '{unknown}'. Known: {KNOWN_TOOLS:?}"
        ));
    }

    let recommends = parse_recommends(&persona_id, raw.recommends)?;
    let name = raw
        .name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(&persona_id)
        .to_string();

    Ok(PersonaManifest {
        id: persona_id,
        name,
        system_prompt: body.trim().to_string(),
        icon: raw.icon.unwrap_or_default().trim().to_string(),
        kind: {
            let declared = raw.kind.as_deref().map(str::trim).filter(|k| !k.is_empty());
            let shell = tools.iter().any(|t| t == "shell");
            let code = raw.family.as_deref() == Some("code");
            match declared {
                // A workbench is not a label you may simply claim. It means "runs a
                // shell, bound to a place", and the GUI acts on that: picking one
                // flips the sidebar to the project list, where an assistant with no
                // folder appears nowhere. Four manifests declared it while having
                // neither a shell nor a code family — copied from a neighbour without
                // reading what the word meant — and the symptom was that clicking
                // Customer Support switched the chat area while the sidebar went to a
                // list of repos: it looked like the click did nothing.
                Some("workbench") if !shell && !code => {
                    return Err(format!(
                        "persona '{}': declares `kind: workbench` but has no `shell` capability and isn't `family: code`. \
                         A workbench runs commands somewhere; drop the line and it will be inferred.",
                        raw.id.as_deref().or(fallback_id).unwrap_or("?")
                    ))
                }
                Some("workbench") => "workbench".to_string(),
                Some("assistant") => "assistant".to_string(),
                // Undeclared: infer from the family, and ONLY from the family.
                //
                // This used to infer `shell || code`, on the reasoning that shell
                // access is what makes something a workbench. It is not. What `kind`
                // decides is where the persona LIVES — a workbench flips the sidebar
                // to the project list and gates a folder; an assistant is always
                // available and needs no folder. That is a statement about being
                // bound to a place, which is what `family: code` means, and shell is
                // a capability that has nothing to say about it.
                //
                // The two only looked equivalent while no assistant had a shell. The
                // moment the knowledge personas were given one (owner ask 2026-09-01,
                // so they can actually run things), the old rule would have silently
                // turned Data Analyst, Finance, HR and Customer Support into
                // workbenches — which is precisely the reported bug where picking
                // Customer Support jumped to the Workspace tab and showed a list of
                // repos it had no place in.
                //
                // `ops` is the case that always disproved the old rule and was simply
                // outvoted by its explicit `kind:` line: knowledge-family, has a
                // shell, and is a workbench because it says so.
                _ => {
                    let _ = shell; // capability, not a binding — see above
                    if code { "workbench".into() } else { "assistant".into() }
                }
            }
        },
        tagline: raw.tagline.unwrap_or_default().trim().to_string(),
        intro: raw.intro.unwrap_or_default().trim().to_string(),
        starters: raw.starters.into_iter().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect(),
        description: raw.description.unwrap_or_default().trim().to_string(),
        tools,
        family,
        workspace,
        messaging: raw.messaging,
        connectors: raw.connectors,
        default_permission_mode: mode,
        needs_capable_model: raw.needs_capable_model,
        skills: clean_list(raw.skills),
        subagents: clean_list(raw.subagents),
        mcp: clean_list(raw.mcp),
        recommends,
        builtin,
        source,
    })
}

pub(in crate::core::personas) fn clean_list(v: Vec<String>) -> Vec<String> {
    v.into_iter().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
}

pub(in crate::core::personas) fn parse_recommends(persona_id: &str, raw: Option<Vec<RawRec>>) -> Result<Vec<Recommendation>, String> {
    let mut out = Vec::new();
    for item in raw.unwrap_or_default() {
        let (kind, reference) = match (item.connector.as_deref(), item.mcp.as_deref()) {
            (Some(c), _) if !c.trim().is_empty() => ("connector".to_string(), c.trim().to_string()),
            (_, Some(m)) if !m.trim().is_empty() => ("mcp".to_string(), m.trim().to_string()),
            _ => {
                return Err(format!(
                    "persona '{persona_id}': each `recommends` item needs a non-empty `connector:` or `mcp:`"
                ))
            }
        };
        let tier = item.tier.as_deref().unwrap_or("optional").trim().to_lowercase();
        if !VALID_TIERS.contains(&tier.as_str()) {
            return Err(format!("persona '{persona_id}': recommend tier must be one of {VALID_TIERS:?}"));
        }
        out.push(Recommendation { kind, reference, reason: item.reason.trim().to_string(), tier });
    }
    Ok(out)
}

pub(in crate::core::personas) fn split_frontmatter(text: &str) -> Result<(RawManifest, String), String> {
    let t = text.strip_prefix('\u{feff}').unwrap_or(text); // tolerate a BOM
    if !t.starts_with("---") {
        return Err("manifest has to open with a `---` YAML frontmatter block".into());
    }
    let after = &t[3..];
    let end = after.find("\n---").ok_or("frontmatter block never closes (no second `---`)")?;
    let raw_yaml = &after[..end];
    let body = after[end + 4..].trim_start_matches('\n').to_string();
    let raw: RawManifest =
        serde_yaml_ng::from_str(raw_yaml).map_err(|e| format!("invalid YAML frontmatter: {e}"))?;
    Ok((raw, body))
}
