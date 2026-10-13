use super::super::*;

pub(in crate::core::agent) fn authoring_tools() -> Vec<ToolSpec> {
    vec![
        tool(
            "list_skills",
            "Every skill on this machine, with the line that says when to use it. Look here before writing a new \
             one: a skill that already does the job is better than a second one that half does it.",
            "authoring",
            RiskClass::Read,
            params(json!({}), &[]),
        ),
        tool(
            "read_skill",
            "Read a skill's full instructions — how one is written, or what the one you would replace already says.",
            "authoring",
            RiskClass::Read,
            params(json!({ "name": {"type": "string", "description": "The skill's name, as list_skills gives it."} }), &["name"]),
        ),
        tool(
            "list_skill_tools",
            "The tool names a skill may declare, grouped by what they are for. Declare only what the skill needs.",
            "authoring",
            RiskClass::Read,
            params(json!({}), &[]),
        ),
        tool(
            "save_skill",
            "Propose a new skill. The user sees it whole and presses a card to add it — nothing is written until \
             they do. Write the instructions in the imperative, as numbered steps, and rehearse one realistic \
             request against them before you propose it. If anything is wrong with the draft you get told, not them.",
            "authoring",
            RiskClass::WriteLocal,
            params(
                json!({
                    "name": {"type": "string", "description": "Lowercase, hyphenated: `weekly-ops-review`."},
                    "description": {"type": "string", "description": "One line: when to reach for this skill."},
                    "instructions": {"type": "string", "description": "The skill itself: what to do, step by step."},
                    "tools": {"type": "array", "items": {"type": "string"}, "description": "Tool names it needs, from list_skill_tools."},
                    "replace": {"type": "boolean", "description": "True only if you mean to replace a skill of the same name you have read."}
                }),
                &["name", "description", "instructions"],
            ),
        ),
        tool(
            "list_personas",
            "Every assistant on this machine: its id, what it is for, what it can use. Look here before proposing \
             a new one — if one already does the job, say so instead.",
            "authoring",
            RiskClass::Read,
            params(json!({}), &[]),
        ),
        tool(
            "read_persona",
            "Read an assistant's standing role — the instructions it works under — and what it was given.",
            "authoring",
            RiskClass::Read,
            params(json!({ "id": {"type": "string", "description": "The assistant's id, as list_personas gives it."} }), &["id"]),
        ),
        tool(
            "list_persona_skills",
            "The skills an assistant can be given, and what each is for.",
            "authoring",
            RiskClass::Read,
            params(json!({}), &[]),
        ),
        tool(
            "save_persona",
            "Propose a new assistant. The user sees it whole and presses a card to add it — nothing is written \
             until they do. Give it as little as it needs: what it may use is what it may do.",
            "authoring",
            RiskClass::WriteLocal,
            params(
                json!({
                    "id": {"type": "string", "description": "Lowercase, hyphenated: `sql-reviewer`."},
                    "name": {"type": "string", "description": "What the person will call it."},
                    "tagline": {"type": "string", "description": "One line: what it is for."},
                    "instructions": {"type": "string", "description": "Its standing role: what it does, how it works, what it refuses."},
                    "tools": {"type": "array", "items": {"type": "string"}, "description": "Capabilities to give it, from the list in list_personas."},
                    "skills": {"type": "array", "items": {"type": "string"}, "description": "Skills it should have, from list_persona_skills."},
                    "mode": {"type": "string", "enum": ["discuss", "plan", "interactive", "auto"], "description": "How far it may act without asking. Default: interactive."},
                    "family": {"type": "string", "enum": ["code", "knowledge"], "description": "Whether it works in a codebase or on documents."}
                }),
                &["id", "name", "tagline", "instructions"],
            ),
        ),
    ]
}

/// Handing work to another assistant.
///
/// Not a catalogue capability, and deliberately: whether a run is offered this depends on
/// who the person let this assistant ask and how far from them the run already is, and the
/// catalogue can see neither. `core::coworker` decides; `run_agent_inner` adds it.
pub fn ask_coworker_spec() -> ToolSpec {
    tool(
        "ask_coworker",
        "Hand a piece of work to another assistant and let it answer for itself. Use it when the work \
         needs a role you don't have — its tools, its access, its judgement — naming what you want done. \
         It answers in its own conversation and the answer comes back to you here. If the work is yours \
         to do, do it; if it needs the person rather than another assistant, ask the person.",
        "coworkers",
        RiskClass::External,
        params(
            json!({
                "coworker": {"type": "string", "description": "Which assistant, by the id or name in your roster."},
                "task": {"type": "string", "description": "What you are asking it to do, in a sentence or two."},
                "constraints": {"type": "string", "description": "Anything that bounds the work: a date range, a system to look in, a rule it must not break."},
                "expecting": {"type": "string", "description": "What a good answer looks like coming back: a list, a number, a recommendation with reasons."}
            }),
            &["coworker", "task"],
        ),
    )
}

pub fn plan_tool_spec() -> ToolSpec {
    tool(
        "run_plan",
        "Run several tool calls in one step when you already know all of them — reading five files, \
         or reading a file and then searching for what it names. Each step runs exactly as if you had \
         called it yourself, with the same checks and approvals. A step can use an earlier step's \
         output: write {{0}} for step 0's result, {{1}} for step 1's. If a step is refused or needs \
         approval, the plan stops there. Use ordinary calls when each step depends on judging the last.",
        "plan",
        RiskClass::Exec,
        params(
            json!({
                "steps": {
                    "type": "array",
                    "description": "Up to 8 calls, run in order. e.g. [{\"tool\":\"read_file\",\"args\":{\"path\":\"a.md\"}},{\"tool\":\"grep\",\"args\":{\"pattern\":\"{{0}}\"}}]",
                    "items": {
                        "type": "object",
                        "properties": {
                            "tool": {"type": "string", "description": "The name of a tool you could call directly."},
                            "args": {"type": "object", "description": "Its arguments, exactly as you would pass them."}
                        },
                        "required": ["tool"]
                    }
                }
            }),
            &["steps"],
        ),
    )
}

/// Every tool the catalogue can ever produce, with the capability it comes from.
///
/// `expand` answers "what does *this session* get", which depends on a workspace and
/// an executor. This answers "what exists at all", which is the question you need to
/// ask which tools are never used — a tool that is never reached because its
/// capability was never available is a different finding from one nobody calls, and
/// the two are indistinguishable without this.
pub fn every_tool() -> Vec<(&'static str, String)> {
    catalog()
        .iter()
        .flat_map(|c| (c.build)().into_iter().map(move |t| (c.id, t.name)))
        .collect()
}

/// Every catalogue tool with its risk class — what `core::gateway` records as
/// `tool.risk`, so a rule can be written against it.
pub fn every_tool_risk() -> Vec<(String, RiskClass)> {
    catalog().iter().flat_map(|c| (c.build)().into_iter().map(|t| (t.name, t.risk))).collect()
}

/// Whether a catalogue tool's schema requires nothing — so a call made with `{}` is a
/// complete call, not arguments that failed to parse.
pub fn takes_no_arguments(name: &str) -> bool {
    catalog()
        .iter()
        .flat_map(|c| (c.build)())
        .find(|t| t.name == name)
        .is_some_and(|t| t.schema["required"].as_array().map_or(true, |req| req.is_empty()))
}

pub fn catalog_info() -> Vec<CapabilityInfo> {
    catalog()
        .iter()
        .map(|c| CapabilityInfo { id: c.id.into(), name: c.name.into(), description: c.description.into(), risk: c.risk.to_vec() })
        .collect()
}
