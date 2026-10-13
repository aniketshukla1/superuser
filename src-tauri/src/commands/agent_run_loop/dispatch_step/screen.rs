use super::*;
use std::sync::atomic::Ordering;

/// A second chance at arguments that would not have worked.
///
/// Both failures handled below — arguments that never parsed, and parsed ones of
/// the wrong shape — are handed back to the model as text, and each costs a step to
/// say. Where the wire speaks `format` that step is avoidable: ask once more with
/// the tool's own schema compiled to a grammar, so the answer cannot be missing a
/// field or carry a string where an integer belongs.
///
pub(super) async fn repair_calls(calls: &mut [(String, String, serde_json::Value, Option<String>)], repair: Option<&crate::core::argrepair::Repair>, specs: &[crate::core::agent::ToolSpec], http: &crate::core::egress::Client, exec: &crate::core::agent_exec::ToolCtx<'_>, cancel: &std::sync::atomic::AtomicBool) {
    if let Some(rep) = repair {
        for call in calls.iter_mut() {
            if cancel.load(Ordering::SeqCst) {
                break;
            }
            // No spec, no schema, nothing to constrain. An unknown tool is `find_tools`'
            // business and a step later, not this one's.
            let Some(spec) = specs.iter().find(|s| s.name == call.1) else { continue };
            let why = match &call.3 {
                Some(unreadable) => unreadable.clone(),
                None => match crate::core::loopshape::check_args(&spec.schema, &call.2) {
                    // The overwhelmingly common case, and it costs one schema walk.
                    Ok(()) => continue,
                    Err(e) => e,
                },
            };
            if let Some(fixed) =
                crate::core::argrepair::attempt(http, rep, spec, &exec.prompt, &call.2, &why).await
            {
                call.2 = fixed;
                call.3 = None;
            }
        }
    }
}

/// Before anything is dispatched.
///
/// Two kinds of call never reach a tool: one asking for a withheld tool, and one
/// whose arguments do not match the schema. The second is HARNESS-PLAN Phase 5's
/// explicit ask and H3 one level up — H3 was about arguments that would not parse,
/// this is about parsed ones of the wrong shape. Dispatched, `write_file` with no
/// path returns some unrelated error from inside the tool and the model spends its
/// next step debugging a problem it does not have.
#[allow(clippy::too_many_arguments)]
pub(super) fn screen_calls(app: &tauri::AppHandle, sink: &Option<EventSink>, calls: &[(String, String, serde_json::Value, Option<String>)], specs: &[crate::core::agent::ToolSpec], held: &[crate::core::agent::ToolSpec], slots: &mut [Option<String>], revealed: &mut Vec<crate::core::agent::ToolSpec>) {
    for (i, (_, name, input, unreadable)) in calls.iter().enumerate() {
        // First, ahead of `find_tools` and ahead of the schema check: a call whose
        // arguments did not parse is not yet a request for anything, so there is
        // nothing to search for and no shape to validate.
        if let Some(why) = unreadable {
            emit_tool_started(app, sink, name, input);
            emit_tool_finished(app, sink, name, why);
            slots[i] = Some(why.clone());
            continue;
        }
        if name == crate::core::loopshape::FIND_TOOLS {
            emit_tool_started(app, sink, name, input);
            let (reply, found) =
                crate::core::loopshape::reveal(held, input["query"].as_str().unwrap_or_default());
            emit_tool_finished(app, sink, name, &reply);
            // Only what is not already on the wire. Two searches that overlap — which
            // is the normal case, since a model that did not get what it wanted asks
            // again in different words — would otherwise send the same schema twice
            // for the rest of the turn.
            for t in found {
                let known = specs.iter().any(|s| s.name == t.name)
                    || revealed.iter().any(|r| r.name == t.name);
                if !known {
                    revealed.push(t);
                }
            }
            slots[i] = Some(reply);
            continue;
        }
        let Some(spec) = specs.iter().find(|s| s.name == *name) else { continue };
        if let Err(e) = crate::core::loopshape::check_args(&spec.schema, input) {
            let out = format!("error: {e}");
            // Emitted like any other call, because it IS one — a step that failed
            // invisibly reads to the user as the model doing nothing.
            emit_tool_started(app, sink, name, input);
            emit_tool_finished(app, sink, name, &out);
            slots[i] = Some(out);
        }
    }
}
