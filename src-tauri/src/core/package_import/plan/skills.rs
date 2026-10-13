use super::*;

/// The package's skills, each with whatever keeps it from importing as it is. Also
/// returns the names that will arrive, which the assistants' skill lists are checked against.
pub(super) fn plan_skills(raw_skills: RawSkills, known_tools: &[String], installed: &[String], left_behind: &mut Vec<String>) -> (Vec<PlannedSkill>, Vec<String>) {
    let mut skills = Vec::new();
    let mut seen_skills: Vec<String> = Vec::new();
    for raw in raw_skills.skills {
        let name = raw.slug.trim().to_string();
        if let Some((_, ours)) = SAME_JOB.iter().find(|(theirs, _)| *theirs == name) {
            left_behind.push(format!(
                "The `{name}` skill: this app's `{ours}` does the same job, so bots that held it get that instead."
            ));
            continue;
        }
        let mut problems = Vec::new();
        if !authoring::is_slug(&name) {
            problems.push(format!("“{}” isn't a name a skill can have here — lowercase letters, digits and hyphens.", one_line(&name)));
        } else if installed.iter().any(|n| *n == name) {
            problems.push(format!("There's already a skill called `{name}` here, and it's kept."));
        } else if seen_skills.contains(&name) {
            problems.push(format!("The package lists `{name}` twice; the first is taken."));
        }
        let description = one_line(&raw.summary);
        if description.is_empty() {
            problems.push("It has no summary, and a summary is what a model picks a skill by.".into());
        } else if description.len() > 200 {
            problems.push("Its summary runs past one line.".into());
        }
        if raw.instructions.trim().len() < 120 {
            problems.push("Its instructions are too short to be a skill.".into());
        }
        let mut tools = Vec::new();
        let mut unmapped = Vec::new();
        for reference in raw.tools {
            let tool = reference.rsplit('/').next().unwrap_or("").trim().to_string();
            if known_tools.iter().any(|k| *k == tool) {
                if !tools.contains(&tool) {
                    tools.push(tool);
                }
            } else {
                unmapped.push(reference.trim().to_string());
            }
        }
        if problems.is_empty() {
            seen_skills.push(name.clone());
        }
        skills.push(PlannedSkill {
            name,
            title: one_line(&raw.title),
            description,
            instructions: raw.instructions.trim().to_string(),
            tools,
            unmapped,
            problems,
        });
    }
    (skills, seen_skills)
}
