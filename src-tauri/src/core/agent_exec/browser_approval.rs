use super::*;

/// Write an approved draft. Reached only after a person pressed the card (or a mode that
/// never asks), so this is the point of no return and does no deciding of its own.
pub(super) fn save(data_dir: &std::path::Path, name: &str, args: &Value) -> Result<String, String> {
    use crate::core::authoring::{self, PersonaDraft, SkillDraft};
    let text = |k: &str| args[k].as_str().unwrap_or("").trim().to_string();
    let list = |k: &str| -> Vec<String> {
        args[k]
            .as_array()
            .map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.trim().to_string())).filter(|s| !s.is_empty()).collect())
            .unwrap_or_default()
    };
    if name == "save_skill" {
        let draft = SkillDraft {
            name: text("name"),
            description: text("description"),
            instructions: args["instructions"].as_str().unwrap_or("").to_string(),
            tools: list("tools"),
            replace: args["replace"].as_bool().unwrap_or(false),
        };
        let preview = crate::core::skills_install::Preview {
            name: draft.name.clone(),
            description: draft.description.clone(),
            instructions: draft.instructions.trim().to_string(),
            allowed_tools: draft.tools.clone(),
            source: "written here".into(),
            url: String::new(),
            replaces_existing: draft.replace,
        };
        let path = crate::core::skills_install::install(data_dir, &preview)?;
        crate::core::audit::log(data_dir, "skill", "", &draft.name, "a skill was added", true);
        Ok(format!(
            "Added the skill `{}` ({}). It is available in every conversation from now on; say what it does and when it will be used.",
            draft.name,
            path.display()
        ))
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
        authoring::save_persona(data_dir, &draft)?;
        crate::core::audit::log(data_dir, "persona", "", &draft.id, "an assistant was added", true);
        Ok(format!(
            "Added the assistant “{}” (`{}`). It is in the assistant list now; say what it is for and what it was given.",
            draft.name, draft.id
        ))
    }
}

/// An engine failure as a tool result. A boundary's `Refused.` keeps its own first word —
/// the transcript draws a refusal apart from a failure — and everything else reads as an
/// error, so the repetition gate and the step's status see it as one.
pub(super) fn browser_failed(e: String) -> String {
    if e.starts_with("Refused.") || e.starts_with("Not done") {
        e
    } else {
        format!("error: {e}")
    }
}

/// Where an "Always allow" given on a page reaches: its host, and its port when the address
/// names one. On this machine especially a port is a different program — a dev server on :3000
/// and a database console on :8080 are not one site, and a grant for the first must not open the
/// second. A scheme's own port (443, 80) is the same site written out.
pub(crate) fn site_of(url: &str) -> String {
    let host = crate::core::boundaries::host_of(url);
    if host.is_empty() {
        return host;
    }
    match reqwest::Url::parse(url).ok().and_then(|u| u.port()) {
        Some(port) if host.contains(':') => format!("[{host}]:{port}"),
        Some(port) => format!("{host}:{port}"),
        None => host,
    }
}

/// Whether the user lets this click or keystroke happen: the approval modes a command
/// meets, asked on the card a command gets. `None` means go ahead.
///
/// Asked per site, not per click. "Always allow" on the card covers acting on that site
/// for the rest of the conversation — a form is a dozen actions, and a card for each is a
/// card nobody reads. A run that has read a page (every run clicking in one) asks even in
/// `auto`, the same taint rule every outside-reaching action meets.
///
/// The reviewer is not consulted: all it would see is `ref=e4 snapshot_id=2`.
pub(super) async fn browser_approval(ctx: &ToolCtx<'_>, name: &str, args: &Value) -> Option<String> {
    use crate::core::agentbrowser::computer;
    use crate::core::cmdgate::grants;
    use crate::core::inbox::{decide, Decision};

    let r = args["ref"].as_str().unwrap_or("").trim();
    let element = if r.is_empty() { None } else { computer::element_for(&ctx.persona, r) };
    let page = element
        .as_ref()
        .map(|(_, url)| url.clone())
        .or_else(|| computer::current_url(&ctx.persona))
        .unwrap_or_default();
    // A password field is refused by the browser itself, with the way round it. Asking the
    // user to approve typing into one first would be asking them to approve the refusal.
    if name == "browser_type" && element.as_ref().is_some_and(|(el, _)| el.input_type == "password") {
        return None;
    }
    let host = crate::core::boundaries::host_of(&page);
    // What "Always allow" reaches: the host, and its port when the address names one.
    let reach = site_of(&page);
    let grant = format!("browser@{reach}");
    if grants::tool_allowed(&ctx.session_id, &grant) {
        return None;
    }
    match decide(name, &host, crate::core::agent::RiskClass::External, &ctx.mode, &ctx.standing, ctx.is_tainted()) {
        Decision::Allow => None,
        Decision::Deny => Some(format!("Not done — this assistant's mode ({}) doesn't act on pages.", ctx.mode)),
        Decision::Ask => {
            let site = if reach.is_empty() { "this page".to_string() } else { reach.clone() };
            // What a person can judge: the control by its label, where it is, and what goes
            // into it. Refs and snapshot ids mean nothing off the page.
            let label = element
                .as_ref()
                .map(|(el, _)| if el.name.is_empty() { format!("the {}", el.role) } else { format!("“{}”", el.name) })
                .unwrap_or_else(|| "the page".into());
            let text = args["text"].as_str().unwrap_or("");
            let key = args["key"].as_str().unwrap_or("").trim();
            let (summary, shown) = match name {
                "browser_click" => (format!("Click {label} on {site}"), serde_json::json!({"click": label, "site": site})),
                "browser_type" => {
                    let then = if args["submit"].as_bool().unwrap_or(false) { "press Enter" } else { "" };
                    let and_then = if then.is_empty() { String::new() } else { format!(" and {then}") };
                    (
                        format!("Type into {label} on {site}{and_then}"),
                        serde_json::json!({"into": label, "text": text, "site": site, "then": then}),
                    )
                }
                _ => (format!("Press {key} on {label} ({site})"), serde_json::json!({"press": key, "on": label, "site": site})),
            };
            let payload = serde_json::json!({
                "name": name,
                "arguments": shown,
                "reason": if ctx.is_tainted() {
                    format!("It read a page this turn, so acting on {site} is confirmed first. “Always allow” covers {site} for the rest of this conversation.")
                } else {
                    format!("Acting on websites is confirmed first. “Always allow” covers {site} for the rest of this conversation.")
                },
                "category": "browser",
            });
            match prompt_user(ctx, "permission", &summary, payload, args).await {
                Ok(d) if d.trim() == "deny" => Some(format!(
                    "Denied by the user — nothing was clicked or typed ({summary}). Don't try it another way; ask what \
                     they'd rather you did."
                )),
                Ok(d) => {
                    if d.trim().starts_with("always") {
                        grants::allow_tool(&ctx.session_id, &grant);
                    }
                    crate::core::audit::log(&ctx.data_dir, "approve", &ctx.session_id, &site, &summary, true);
                    None
                }
                Err(e) => Some(e),
            }
        }
    }
}

/// How long a request for the user's hands waits. The same half hour as a question: the
/// thing being waited on is a person, who may have stepped away.
pub(super) const HANDS_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30 * 60);
