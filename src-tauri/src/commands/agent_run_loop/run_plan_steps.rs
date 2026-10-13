use super::super::*;

/// A tool call finishing. The preview is capped: it feeds the step's raw drawer, not
/// the model.
pub(in crate::commands) fn emit_tool_finished(app: &tauri::AppHandle, sink: &Option<EventSink>, name: &str, out: &str) {
    use serde_json::json;
    use tauri::Emitter;
    let payload = json!({
        "type": "tool_done",
        "name": name,
        "status": tool_status(out),
        "text": out.chars().take(600).collect::<String>(),
    });
    let _ = app.emit("agent-run", payload.clone());
    if let Some(tx) = sink {
        let _ = tx.send(payload);
    }
}

/// One step's tool calls: reads together, everything else in order.
///
/// Returns `tool_result` blocks in the MODEL'S emitted order regardless of the order
/// they finished in. That is not cosmetic — the API pairs each result to its
/// `tool_use` by id, and a transcript whose results are shuffled relative to the calls
/// is one nobody can read afterwards either.
///
/// Three properties this holds, and each is a thing the plan says must not change:
///
/// * **Approvals stay sequential.** Everything that can ask the user is in the serial
///   group, so two prompts can never be in flight at once.
/// * **Taint order is unchanged.** Reads cannot taint each other — they change
///   nothing — and everything that can runs after them, in order.
/// * **The ledger sees the same sequence.** Results are recorded in emitted order.
#[allow(clippy::too_many_arguments)]
/// Read a tool call's arguments off an OpenAI-compatible wire.
///
/// This format sends arguments as a JSON *string*, so it has a failure the Claude
/// format does not: the string may not be JSON. It happens for real — a model that
/// hits its output cap mid-argument sends a truncated one, and small local models
/// produce malformed ones unprompted.
///
/// This used to be `unwrap_or(json!({}))`. The call then ran with no arguments at
/// all: `write_file` with no path, `run_shell` with no command. The tool returned an
/// error about the missing field, the model read an error about something it had not
/// done, and the next step went into debugging a problem that did not exist. The
/// parse failure was the one fact that would have fixed it, and it was the one fact
/// thrown away.
///
/// Returning it means it can be handed back as that call's own result, which is what
/// gets the call retried rather than mourned. `{}` is still returned alongside so the
/// call keeps a well-formed shape for the event the UI renders.
pub(in crate::commands) fn read_arguments(raw: &serde_json::Value) -> (serde_json::Value, Option<String>) {
    match raw {
        serde_json::Value::String(s) => match serde_json::from_str(s) {
            Ok(v) => (v, None),
            Err(e) => (
                serde_json::json!({}),
                Some(format!(
                    "error: the arguments for this call were not valid JSON ({e}). \
                     Call the tool again with its arguments as one complete JSON object."
                )),
            ),
        },
        o => (o.clone(), None),
    }
}

/// Run a plan's steps in order, each through `run_tool` exactly as a single call.
///
/// A later step may use an earlier one's output by writing `{{N}}`. A step that is
/// refused or parked ends the plan, because the steps after it were written assuming
/// it worked — nothing runs past a gate.
pub(in crate::commands) async fn run_plan_steps(
    app: &tauri::AppHandle,
    state: &AppState,
    exec: &crate::core::agent_exec::ToolCtx<'_>,
    skills: &crate::core::skills::SkillStore,
    mcp: &mut McpBundle,
    http: &crate::core::egress::Client,
    depth: u8,
    args: &serde_json::Value,
) -> String {
    if !state.settings.lock().unwrap().plan_tool_enabled {
        return "error: run_plan is off. Make the calls one at a time.".into();
    }
    let steps = args["steps"].as_array().cloned().unwrap_or_default();
    if steps.is_empty() {
        return "error: give at least one step".into();
    }
    if steps.len() > 8 {
        return format!("error: {} steps is too many for one plan — at most 8", steps.len());
    }
    let mut results: Vec<String> = Vec::new();
    let mut report = String::new();
    for (i, step) in steps.iter().enumerate() {
        let tool = step["tool"].as_str().unwrap_or("").trim().to_string();
        if tool.is_empty() || tool == "run_plan" {
            report.push_str(&format!(
                "step {i}: error — {}\n",
                if tool.is_empty() { "no tool named" } else { "a plan cannot contain a plan" }
            ));
            break;
        }
        let step_args = plan_substitute(&step["args"], &results);
        let out = run_tool(app, state, exec, skills, &mut *mcp, http, depth, &tool, &step_args).await;
        report.push_str(&format!("step {i} — {tool}:\n{}\n\n", out.chars().take(4000).collect::<String>()));
        let head = out.trim_start().to_lowercase();
        let halted = ["error", "blocked", "parked for approval", "not run", "refused."].iter().any(|h| head.starts_with(h));
        results.push(out);
        if halted {
            report.push_str(&format!("The plan stopped at step {i}; the steps after it did not run.\n"));
            break;
        }
    }
    report
}
