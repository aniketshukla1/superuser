use super::super::*;

/// Expand a persona's capability id list into the concrete tool specs available in
/// this session context (skipping capabilities whose prerequisites aren't met).
pub fn expand(ids: &[String], ctx: &AgentContext) -> Vec<ToolSpec> {
    let cat = catalog();
    let mut out = Vec::new();
    let ids: Vec<String> = ALWAYS_ON
        .iter()
        .map(|s| s.to_string())
        .chain(ids.iter().filter(|i| !ALWAYS_ON.contains(&i.as_str())).cloned())
        .collect();
    for id in &ids {
        if let Some(cap) = cat.iter().find(|c| c.id == id.as_str()) {
            if cap.available(ctx) {
                out.extend((cap.build)());
            }
        }
    }
    out
}

/// The union of risk classes a capability id list can produce — for consent screens.
pub fn risk_summary(ids: &[String]) -> Vec<RiskClass> {
    let cat = catalog();
    let mut out: Vec<RiskClass> = Vec::new();
    for id in ids {
        if let Some(cap) = cat.iter().find(|c| c.id == id.as_str()) {
            for r in cap.risk {
                if !out.contains(r) {
                    out.push(*r);
                }
            }
        }
    }
    out
}

/// Catalog view for the UI ("what can this assistant do").
#[derive(Debug, Clone, Serialize)]
pub struct CapabilityInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub risk: Vec<RiskClass>,
}

/// `run_plan` — several tool calls in one step, each governed exactly as if the model
/// had made it itself.
///
/// The measured benefit of "code mode" is fewer model round trips: Anthropic's own
/// workflow went from 150k to 2k tokens, and on a local model a round trip is a full
/// prefill of the whole prompt, tens of seconds each. The cost of code mode as usually
/// built is arbitrary generated code running against the tool surface, which is the
/// one thing this app exists to govern.
///
/// This takes the benefit and refuses the cost. A plan is a list of ordinary tool
/// calls — no code, no interpreter — and every step goes through `run_tool_with`, the
/// same path a single call takes: the gates, the permission check, the taint. A later
/// step may use an earlier one's output by writing `{{N}}` in an argument. A step that
/// is refused or parked stops the plan there, so nothing runs past a gate.
///
/// Off unless the user turns it on (`plan_tool_enabled`), `RiskClass::Exec` so the
/// read-only prose salvage can never run one, and never nested.
/// A view the model lays out itself — G1's other half.
///
/// Not a catalogue capability: whether it is offered depends on a setting, and the
/// catalogue cannot see settings. `run_agent_inner` adds it when the person has turned it
/// on, along with `core::genui::GUIDANCE`.
pub fn ui_component_spec() -> ToolSpec {
    tool(
        "ui_component",
        "Draw a small view in the conversation — HTML and CSS you write yourself — for an answer whose shape none of \
         `card`, `table` or `chart` fits. It runs sealed: no network, no access to this app, and nothing in it can send \
         anything anywhere, so put the values in the markup rather than fetching them. Say what it shows in your answer \
         too: somebody reading later may have only your words.",
        "genui",
        RiskClass::Read,
        params(
            json!({
                "title": {"type": "string", "description": "What this view is, shown above it."},
                "html": {"type": "string", "description": "The markup. Plain HTML and CSS, inline, with the values already in it."},
                "source": {"type": "string", "description": "Where its numbers came from — checked against what this run read, like a chart's."}
            }),
            &["title", "html", "source"],
        ),
    )
}

/// Making a skill or an assistant by talking to one — D6/E1.
///
/// Looking is free; saving is a card the person presses. The `skill-creator` and
/// `persona-creator` skills do the interviewing and call these.
/// The automations a person already has. `schedule_task` makes one; these three are the
/// rest of the verbs, and without them an assistant could create a routine and then not
/// answer "what have I got running?".
pub(in crate::core::agent) fn automation_tools() -> Vec<ToolSpec> {
    vec![
        tool(
            "list_automations",
            "Every automation on this machine: what it does, when it runs, whether it is on, and how the last run went. \
             Read this before changing or cancelling one — the id comes from here.",
            "schedule",
            RiskClass::Read,
            params(json!({}), &[]),
        ),
        tool(
            "update_automation",
            "Change an automation: its name, what it does when it fires, when it runs, or whether it is switched on. \
             Say only what changes. Switching one off keeps its history and is usually what someone means by \"stop it\".",
            "schedule",
            RiskClass::WriteLocal,
            params(
                json!({
                    "id": {"type": "string", "description": "From list_automations."},
                    "title": {"type": "string", "description": "A new name."},
                    "instructions": {"type": "string", "description": "What one firing should do, in the imperative. Don't restate the schedule."},
                    "cron": {"type": "string", "description": "A new schedule, five cron fields — e.g. `0 9 * * 1-5`."},
                    "timezone": {"type": "string", "description": "A zone name like Europe/Madrid, UTC, or an offset like +02:00. Leave out for this machine's clock."},
                    "enabled": {"type": "boolean", "description": "False switches it off and keeps it."}
                }),
                &["id"],
            ),
        ),
        tool(
            "delete_automation",
            "Delete an automation and its run history. Prefer switching it off — that keeps both and can be undone. \
             Deleting needs confirm: true, which you set only when the person has said they mean delete.",
            "schedule",
            RiskClass::WriteLocal,
            params(
                json!({
                    "id": {"type": "string", "description": "From list_automations."},
                    "confirm": {"type": "boolean", "description": "True only when the person has said they want it gone, history and all."}
                }),
                &["id"],
            ),
        ),
    ]
}
