use super::*;

/// The withheld tools matching `query`, best first.
///
/// Scored on words rather than on the whole string: a model asking to "read the
/// spreadsheet numbers" should reach `sheet_read`, and a substring match on the
/// sentence finds nothing at all.
pub fn find(held: &[ToolSpec], query: &str, limit: usize) -> Vec<ToolSpec> {
    let words = words_of(query);
    let mut scored: Vec<(usize, &ToolSpec)> = held
        .iter()
        .filter_map(|t| {
            let name_words = words_of(&t.name);
            let prose_words = words_of(&format!("{} {}", t.description, t.capability));
            let score: usize = words
                .iter()
                .map(|w| {
                    // A hit in the name is worth more than one in the prose: the name
                    // is what the model will have to emit.
                    if name_words.iter().any(|n| akin(w, n)) {
                        3
                    } else if prose_words.iter().any(|n| akin(w, n)) {
                        1
                    } else {
                        0
                    }
                })
                .sum();
            (score > 0).then_some((score, t))
        })
        .collect();
    // Stable within a score, so the same query returns the same tools in the same
    // order — a shifting answer to a repeated question is how a model gets stuck.
    scored.sort_by(|a, b| b.0.cmp(&a.0));
    scored.into_iter().take(limit).map(|(_, t)| t.clone()).collect()
}

/// The reply `find_tools` gives the model, and the specs the loop should now send.
pub fn reveal(held: &[ToolSpec], query: &str) -> (String, Vec<ToolSpec>) {
    let found = find(held, query, 5);
    if found.is_empty() {
        let mut caps: Vec<&str> = held.iter().map(|t| t.capability.as_str()).collect();
        caps.sort_unstable();
        caps.dedup();
        return (
            format!(
                "No tool matches that. What is available beyond the listed tools: {}.",
                caps.join(", ")
            ),
            Vec::new(),
        );
    }
    let names: Vec<String> = found
        .iter()
        .map(|t| format!("- {}: {}", t.name, t.description))
        .collect();
    (
        format!(
            "These tools are now available to you and you can call them directly:\n{}",
            names.join("\n")
        ),
        found,
    )
}

// ---------------------------------------------------------------------------
// Argument validation, before dispatch
// ---------------------------------------------------------------------------

/// Check a tool call's arguments against the tool's own schema.
///
/// HARNESS-PLAN Phase 5 asks for this on every tier, and it is H3 one level up. H3
/// was about *unparseable* arguments being replaced with `{}`; this is about
/// parseable ones that are the wrong shape — `write_file` with no `path`,
/// `run_shell` with a `command` that arrived as a list. Dispatched, those produce
/// some other error from inside the tool, and the model spends its next step
/// debugging a problem it does not have.
///
/// Deliberately partial. `required` and `type` are what the schemas in this app
/// actually constrain and what models actually get wrong; enum, pattern, minimum and
/// the rest are not checked, because a validator that half-implements a spec and
/// claims otherwise is worse than one that says what it does.
pub fn check_args(schema: &Value, args: &Value) -> Result<(), String> {
    let Some(obj) = args.as_object() else {
        return Err(format!(
            "arguments must be a JSON object, got {}",
            type_name(args)
        ));
    };
    let mut problems = Vec::new();
    for r in schema["required"].as_array().into_iter().flatten() {
        if let Some(name) = r.as_str() {
            if !obj.contains_key(name) || obj[name].is_null() {
                problems.push(format!("missing required argument \"{name}\""));
            }
        }
    }
    if let Some(props) = schema["properties"].as_object() {
        for (name, value) in obj {
            let Some(spec) = props.get(name) else { continue };
            let Some(want) = spec["type"].as_str() else { continue };
            if !type_matches(want, value) {
                problems.push(format!(
                    "\"{name}\" must be {want}, got {}",
                    type_name(value)
                ));
            }
        }
    }
    if problems.is_empty() {
        return Ok(());
    }
    // Named, so the model can fix the call rather than guess which argument was
    // wrong — the whole point of handing this back instead of failing inside the tool.
    Err(format!(
        "invalid arguments: {}. Required: {}.",
        problems.join("; "),
        required_summary(schema)
    ))
}

pub(super) fn type_matches(want: &str, v: &Value) -> bool {
    match want {
        "string" => v.is_string(),
        // An integer is a number; a number with a fraction is not an integer.
        "integer" => v.is_i64() || v.is_u64(),
        "number" => v.is_number(),
        "boolean" => v.is_boolean(),
        "array" => v.is_array(),
        "object" => v.is_object(),
        // A type this validator does not know is not a type this validator rejects.
        _ => true,
    }
}

pub(super) fn type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

pub(super) fn required_summary(schema: &Value) -> String {
    let props = schema["properties"].as_object();
    let names: Vec<String> = schema["required"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|r| r.as_str())
        .map(|n| {
            let ty = props
                .and_then(|p| p.get(n))
                .and_then(|s| s["type"].as_str())
                .unwrap_or("value");
            format!("{n} ({ty})")
        })
        .collect();
    if names.is_empty() {
        "none".into()
    } else {
        names.join(", ")
    }
}
