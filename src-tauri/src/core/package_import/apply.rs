use super::*;

/// Import what a preview showed, if the folder still says the same thing.
pub fn apply(data_dir: &Path, dir: &Path, digest: &str) -> Result<Applied, String> {
    let reg = PersonaRegistry::load(data_dir);
    let store = SkillStore::load(data_dir);
    let plan = plan(dir, &reg, &store)?;
    if plan.digest != digest {
        return Err("That package changed after it was read. Read it again, and import what it says now.".into());
    }
    let source = dir.display().to_string();
    let mut written_skills = Vec::new();
    for skill in plan.skills.iter().filter(|s| s.problems.is_empty()) {
        let preview = crate::core::skills_install::Preview {
            name: skill.name.clone(),
            description: skill.description.clone(),
            instructions: skill.instructions.clone(),
            allowed_tools: skill.tools.clone(),
            source: "Agent package".into(),
            url: source.clone(),
            replaces_existing: false,
        };
        crate::core::skills_install::install(data_dir, &preview)?;
        written_skills.push(skill.name.clone());
    }
    // Switched off, as the assistants are: pressing Import is reading the package, not asking for
    // every generalist to be offered its procedures.
    let mut store = SkillStore::load(data_dir);
    for name in &written_skills {
        store.set_on(name, false)?;
    }

    let folder = data_dir.join("personas");
    std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    let mut written = Vec::new();
    for p in plan.personas.iter().filter(|p| p.problems.is_empty()) {
        let draft = PersonaDraft {
            id: p.id.clone(),
            name: p.name.clone(),
            tagline: p.tagline.clone(),
            instructions: p.instructions.clone(),
            tools: p.tools.clone(),
            skills: p.skills.clone(),
            mode: "interactive".into(),
            family: "knowledge".into(),
        };
        crate::core::store::write_atomic(&folder.join(format!("{}.md", p.id)), &authoring::manifest_text(&draft)).map_err(|e| e.to_string())?;
        written.push(p);
    }
    // Re-read, so the cards describe what the registry now holds — and set remote addresses
    // on the assistants that have one. Nothing is switched on.
    let mut reg = PersonaRegistry::load(data_dir);
    let mut consent = Vec::new();
    for p in written {
        if let Some(address) = &p.remote {
            reg.set_remote(&p.id, address)?;
        }
        if let Some(m) = reg.manifest(&p.id) {
            // Where it came from, on the card: the registry's own `source` is the file it
            // was written to, and "who wrote this" is a question about the package.
            consent.push(crate::core::personas_install::Installed {
                source: Some(format!("the agent package at {source}")),
                ..crate::core::personas_install::Installed::from(m)
            });
        }
    }
    crate::core::audit::log(
        data_dir,
        "import",
        "user",
        &source,
        &format!("Agent package: {} assistants and {} skills, all switched off", consent.len(), written_skills.len()),
        true,
    );
    Ok(Applied { skills: written_skills, consent })
}
