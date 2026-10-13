use super::*;

/// Writing a skill or an assistant — D6/E1.
///
/// Looking is free. Saving goes to the Inbox like a send does, and for the same reason: it
/// is the person's decision, and what they approve is the finished draft rather than an
/// argument about it. A draft that doesn't check out never gets that far — the problems go
/// back to the model that wrote it, which is the only party that can fix them.
pub(super) async fn authoring(ctx: &ToolCtx<'_>, name: &str, args: &Value) -> String {
    use crate::core::authoring::{self, PersonaDraft, SkillDraft};
    use crate::core::inbox::{decide, Ask, Decision, Inbox};
    use crate::core::personas::PersonaRegistry;
    use crate::core::skills::SkillStore;

    let text = |k: &str| args[k].as_str().unwrap_or("").trim().to_string();
    let list = |k: &str| -> Vec<String> {
        args[k]
            .as_array()
            .map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.trim().to_string())).filter(|s| !s.is_empty()).collect())
            .unwrap_or_default()
    };
    let store = SkillStore::load(&ctx.data_dir);

    match name {
        "list_skills" => {
            let mut out = String::from("Skills on this machine:\n\n");
            for card in store.catalog() {
                let builtin = store.get(&card.name).is_some_and(|s| s.path.is_none());
                out.push_str(&format!("- `{}`{} — {}\n", card.name, if builtin { " (built in)" } else { "" }, card.description));
            }
            out
        }
        "read_skill" => {
            let n = text("name");
            match store.get(&n) {
                Some(s) => format!(
                    "# {}\n\n{}\n\nTools it declares: {}\n\n{}",
                    s.name,
                    s.description,
                    if s.allowed_tools.is_empty() { "none".into() } else { s.allowed_tools.join(", ") },
                    s.instructions
                ),
                None => format!("error: there's no skill called `{n}`. Call list_skills to see what this machine has."),
            }
        }
        "list_skill_tools" => {
            let mut out = String::from("Tool names a skill may declare, by what they are for:\n\n");
            for (cap, tools) in authoring::tools_by_capability() {
                out.push_str(&format!("- **{cap}**: {}\n", tools.join(", ")));
            }
            out.push_str("\nDeclare only what the skill actually needs.");
            out
        }
        "list_personas" => {
            let reg = PersonaRegistry::load(&ctx.data_dir);
            let mut out = String::from("Assistants on this machine:\n\n");
            for p in reg.list_all() {
                out.push_str(&format!(
                    "- `{}` — {}{}\n  can use: {}\n",
                    p.id,
                    if p.tagline.is_empty() { p.name.clone() } else { p.tagline.clone() },
                    if p.enabled { "" } else { " (switched off)" },
                    if p.tools.is_empty() { "nothing but the always-on tools".to_string() } else { p.tools.join(", ") }
                ));
            }
            out.push_str(&format!(
                "\nWhat a new one can be given: {}.",
                crate::core::personas::GRANTABLE_TOOLS.join(", ")
            ));
            out
        }
        "read_persona" => {
            let id = text("id");
            let reg = PersonaRegistry::load(&ctx.data_dir);
            match reg.manifest(&id) {
                Some(m) => format!(
                    "# {} (`{}`)\n\n{}\n\nMode: {}\nCan use: {}\nSkills: {}\n\n## Its standing role\n\n{}",
                    m.name,
                    m.id,
                    m.tagline,
                    m.default_permission_mode,
                    if m.tools.is_empty() { "nothing but the always-on tools".to_string() } else { m.tools.join(", ") },
                    if m.skills.is_empty() { "everything installed".to_string() } else { m.skills.join(", ") },
                    m.system_prompt
                ),
                None => format!("error: there's no assistant with the id `{id}`. Call list_personas to see who is here."),
            }
        }
        "list_persona_skills" => {
            let mut out = String::from("Skills an assistant can be given:\n\n");
            for card in store.catalog() {
                out.push_str(&format!("- `{}` — {}\n", card.name, card.description));
            }
            out.push_str("\nGiving none means it gets whatever is installed.");
            out
        }
        "save_skill" | "save_persona" => {
            let (target, summary, problems) = if name == "save_skill" {
                let draft = SkillDraft {
                    name: text("name"),
                    description: text("description"),
                    instructions: args["instructions"].as_str().unwrap_or("").to_string(),
                    tools: list("tools"),
                    replace: args["replace"].as_bool().unwrap_or(false),
                };
                let problems = authoring::check_skill(&store, &draft);
                (draft.name.clone(), format!("Add the skill “{}”", draft.name), problems)
            } else {
                let draft = PersonaDraft {
                    id: text("id"),
                    name: text("name"),
                    tagline: text("tagline"),
                    instructions: args["instructions"].as_str().unwrap_or("").to_string(),
                    tools: list("tools"),
                    skills: list("skills"),
                    mode: text("mode"),
                    family: text("family"),
                };
                let reg = PersonaRegistry::load(&ctx.data_dir);
                let problems = authoring::check_persona(&reg, &store, &draft);
                (draft.id.clone(), format!("Add the assistant “{}”", draft.name), problems)
            };
            if !problems.is_empty() {
                // To the model, in its own words, and never to the person: they have not
                // read this draft and cannot fix it.
                return format!(
                    "Not proposed — fix these and call it again:\n\n{}",
                    problems.iter().map(|p| format!("- {p}")).collect::<Vec<_>>().join("\n")
                );
            }
            match decide(name, &target, crate::core::agent::RiskClass::WriteLocal, &ctx.mode, &ctx.standing, ctx.is_tainted()) {
                Decision::Deny => format!(
                    "Not proposed — this assistant's mode ({}) doesn't write anything. Say what you would have written instead.",
                    ctx.mode
                ),
                Decision::Allow => match save(&ctx.data_dir, name, args) {
                    Ok(where_it_went) => where_it_went,
                    Err(e) => format!("error: {e}"),
                },
                Decision::Ask => {
                    let ask = Inbox::load(&ctx.data_dir).park(Ask::new(
                        &ctx.session_id,
                        ctx.task_id.clone(),
                        name,
                        &target,
                        &summary,
                        args.clone(),
                    ));
                    format!(
                        "Put to the user (inbox id {}). Nothing has been written — they see the whole thing and press the card.{}",
                        ask.id,
                        if ctx.unattended { "" } else { " Tell them it's waiting, and what it does." }
                    )
                }
            }
        }
        other => format!("unknown tool: {other}"),
    }
}
