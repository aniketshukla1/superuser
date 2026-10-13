use super::*;

mod skills;
use skills::*;

/// Read a package folder into what an import would do. Writes nothing.
pub fn plan(dir: &Path, reg: &PersonaRegistry, store: &SkillStore) -> Result<Plan, String> {
    if !dir.is_dir() {
        return Err("That isn't a folder.".into());
    }
    let mut hasher = Sha256::new();
    let agents_text = read_file(dir, "agents.yaml", &mut hasher)?;
    let skills_text = read_file(dir, "skills.yaml", &mut hasher)?;
    if agents_text.is_none() && skills_text.is_none() {
        return Err("That folder has no agents.yaml or skills.yaml, so it isn't an agent package.".into());
    }
    let knowledge_text = read_file(dir, "knowledge.yaml", &mut hasher)?;
    let model_text = read_file(dir, "model.yaml", &mut hasher)?;

    let agents: RawAgents = match &agents_text {
        Some(t) => serde_yaml_ng::from_str(t).map_err(|e| format!("agents.yaml doesn't read as a package file: {e}"))?,
        None => RawAgents::default(),
    };
    let raw_skills: RawSkills = match &skills_text {
        Some(t) => serde_yaml_ng::from_str(t).map_err(|e| format!("skills.yaml doesn't read as a package file: {e}"))?,
        None => RawSkills::default(),
    };
    if agents.agents.len() > MAX_AGENTS {
        return Err(format!("That package offers {} bots. More than {MAX_AGENTS} isn't something anyone reviews — split it.", agents.agents.len()));
    }
    if raw_skills.skills.len() > MAX_SKILLS {
        return Err(format!("That package offers {} skills, more than the {MAX_SKILLS} an import takes.", raw_skills.skills.len()));
    }

    let mut left_behind = Vec::new();
    let known_tools = authoring::tool_names();
    let installed = store.names();

    let (skills, seen_skills) = plan_skills(raw_skills, &known_tools, &installed, &mut left_behind);

    // ---- assistants ----
    let mut personas = Vec::new();
    let mut seen_ids: Vec<String> = Vec::new();
    for raw in agents.agents {
        let id = raw.id.trim().to_string();
        let name = one_line(&raw.name);
        let title = one_line(&raw.title);
        let role = one_line(&raw.role_description);
        let who = if title.is_empty() { format!("You are {name}.") } else { format!("You are {name}, {title}.") };
        let remote = raw.kind.trim() == "remote-ag-ui";
        let mut notes = Vec::new();
        let mut problems = Vec::new();

        let endpoint = if remote {
            let address = without_environment(&raw.endpoint);
            if address.is_empty() {
                left_behind.push(format!(
                    "{}: it runs on a remote agent whose address the package leaves to the deployment. Make an assistant and set its address under Where it runs.",
                    if name.is_empty() { id.clone() } else { name.clone() }
                ));
                continue;
            }
            if let Some(problem) = crate::core::aguiclient::endpoint_problem(&address) {
                problems.push(format!("Its remote agent's address won't do: {problem}"));
            }
            Some(address)
        } else {
            if raw.kind.trim() != "built-in" && !raw.kind.trim().is_empty() {
                notes.push(format!("The package calls it a `{}` bot; it arrives as an ordinary assistant.", one_line(&raw.kind)));
            }
            None
        };

        let instructions = match &endpoint {
            Some(address) => format!(
                "{who} {role}\n\nYour turns are run by a remote agent at {}. What that agent is told is its own business; this is what this app says about you.",
                crate::core::boundaries::host_of(address)
            ),
            None => format!("{who} {role}\n\n{}", raw.system_prompt.trim()),
        };

        let mut tools: Vec<String> = if endpoint.is_some() { Vec::new() } else { IMPORTED_TOOLS.iter().map(|t| t.to_string()).collect() };
        let mut kept_skills = Vec::new();
        for wanted in &raw.skills {
            let wanted = wanted.trim();
            let (resolved, authoring_skill) = match SAME_JOB.iter().find(|(theirs, _)| *theirs == wanted) {
                Some((_, ours)) => (ours.to_string(), true),
                None => (wanted.to_string(), false),
            };
            let arrives = installed.iter().any(|n| *n == resolved) || seen_skills.contains(&resolved);
            if !arrives {
                notes.push(format!("Its skill `{wanted}` isn't coming across, so it goes without."));
                continue;
            }
            if endpoint.is_some() {
                notes.push(format!("Its skill `{wanted}` is dropped: a remote agent's skills are its own."));
                continue;
            }
            if authoring_skill && !tools.iter().any(|t| t == "authoring") {
                tools.push("authoring".into());
                notes.push(format!("It held `{wanted}`, so it gets this app's `{resolved}` and the authoring tools that come with it."));
            }
            if !kept_skills.contains(&resolved) {
                kept_skills.push(resolved);
            }
        }

        if !authoring::is_slug(&id) {
            problems.push(format!("“{}” isn't an id an assistant can have here — lowercase letters, digits and hyphens.", one_line(&id)));
        } else if reg.manifest(&id).is_some() {
            problems.push(format!("There's already an assistant with the id `{id}` here, and it's kept."));
        } else if seen_ids.contains(&id) {
            problems.push(format!("The package lists `{id}` twice; the first is taken."));
        }
        if name.is_empty() {
            problems.push("It has no name.".into());
        }
        let tagline = if !title.is_empty() { title.clone() } else { role.clone() };
        let draft = PersonaDraft {
            id: id.clone(),
            name: name.clone(),
            tagline: tagline.clone(),
            instructions: instructions.clone(),
            tools: tools.clone(),
            skills: kept_skills.clone(),
            mode: "interactive".into(),
            family: "knowledge".into(),
        };
        // The registry's own parser has the last word, as it does for anything written here.
        if problems.is_empty() {
            if let Err(e) = personas::parse_manifest(&authoring::manifest_text(&draft), Some(&id), false, None) {
                problems.push(format!("It didn't come out as a valid assistant: {e}"));
            }
        }
        if problems.is_empty() {
            seen_ids.push(id.clone());
        }
        personas.push(PlannedPersona { id, name, tagline, instructions, tools, skills: kept_skills, remote: endpoint, problems, notes });
    }

    // ---- the deployment's own business ----
    if dir.join("channels.yaml").exists() {
        left_behind.push("Channels: they decide which groups of people may reach which bots, and this app has one person.".into());
    }
    if dir.join("brand.yaml").exists() || dir.join("theme.css").exists() {
        left_behind.push("Branding and theme: they are the deployment's own look, and this app keeps its own.".into());
    }
    if let Some(text) = &model_text {
        let provider = serde_yaml_ng::from_str::<RawModel>(text).map(|m| one_line(&m.model.provider)).unwrap_or_default();
        left_behind.push(format!(
            "The model{}: models and keys are chosen in Settings ▸ Models, and a package never sets them.",
            if provider.is_empty() { String::new() } else { format!(" ({provider})") }
        ));
    }
    if let Some(text) = &knowledge_text {
        let mut kinds: Vec<String> = serde_yaml_ng::from_str::<RawSources>(text)
            .map(|k| k.sources.into_iter().map(|s| one_line(&s.kind)).filter(|s| !s.is_empty()).collect())
            .unwrap_or_default();
        kinds.dedup();
        left_behind.push(format!(
            "Document sources{}: connect them under Connected apps to give these assistants the same reach.",
            if kinds.is_empty() { String::new() } else { format!(" ({})", kinds.join(", ")) }
        ));
    }

    let digest = hasher.finalize().iter().map(|b| format!("{b:02x}")).collect::<String>();
    Ok(Plan { digest, personas, skills, left_behind })
}
