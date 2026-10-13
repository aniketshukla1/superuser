use super::super::*;

#[tauri::command]
pub fn set_mascot_visible(app: tauri::AppHandle, visible: bool) {
    use tauri::Manager;
    if let Some(w) = app.get_webview_window("mascot") {
        let _ = if visible { w.show() } else { w.hide() };
    }
}

/// Restore + focus the main window (called when the mascot is clicked) and hide
/// the mascot.
#[tauri::command]
pub fn show_main_window(app: tauri::AppHandle) {
    use tauri::Manager;
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.unminimize();
        let _ = main.show();
        let _ = main.set_focus();
    }
    if let Some(mascot) = app.get_webview_window("mascot") {
        let _ = mascot.hide();
    }
}

/// Push a message/notification to the mascot bubble (e.g. a reminder while the
/// app is minimized). The mascot window listens for the `mascot-message` event.
#[tauri::command]
pub fn notify_mascot(app: tauri::AppHandle, message: String) {
    use tauri::{Emitter, Manager};
    if let Some(mascot) = app.get_webview_window("mascot") {
        let _ = mascot.emit("mascot-message", message);
    }
}

/// Record what the user thought of one assistant turn.
///
/// The evidence half is filled in here rather than sent by the front end: the model,
/// the persona and the tools that ran are facts this process already holds, and a
/// client that supplied them could get them wrong or stale. The user supplies the
/// judgement; we supply what happened.
#[tauri::command]
pub fn feedback_record(
    state: State<'_, AppState>,
    session: String,
    message: usize,
    verdict: String,
    reason: Option<String>,
    note: Option<String>,
) -> Result<(), String> {
    use crate::core::feedback;
    let verdict = match verdict.as_str() {
        "up" => feedback::Verdict::Up,
        "down" => feedback::Verdict::Down,
        other => return Err(format!("verdict must be \"up\" or \"down\", not {other:?}")),
    };
    // A downvote with no reason is the question-with-no-options failure one surface
    // over: it costs the user a click and produces a number nobody can act on.
    let reason = match verdict {
        feedback::Verdict::Down => Some(
            reason
                .as_deref()
                .and_then(feedback::Reason::parse)
                .ok_or("a downvote needs a reason — pick one of the offered ones")?,
        ),
        feedback::Verdict::Up => None,
    };

    let mut e = feedback::entry(&session, message, verdict);
    e.reason = reason;
    e.note = note.unwrap_or_default().trim().chars().take(2_000).collect();

    if let Some(s) = crate::core::sessions::SessionStore::new(&state.data_dir).get(&session) {
        e.evidence.persona = s.agent.clone();
        e.evidence.model = s.model.clone();
        if let Some(m) = s.messages.get(message) {
            e.message_at = m.at;
            // Tools this turn actually called, in order and deduplicated.
            if let Some(wire) = &m.wire {
                let mut seen: Vec<String> = Vec::new();
                for name in wire.tool_names() {
                    if !seen.iter().any(|s| *s == name) {
                        seen.push(name);
                    }
                }
                e.evidence.tools = seen;
            }
        }
    }
    feedback::record(&state.data_dir, &e);
    Ok(())
}

/// Every verdict the user has given in one session, so the transcript can show which
/// messages they have already judged.
#[tauri::command]
pub fn feedback_for_session(state: State<'_, AppState>, session: String) -> Vec<(usize, String)> {
    let all = crate::core::feedback::all(&state.data_dir);
    let mut out: std::collections::BTreeMap<usize, String> = Default::default();
    for e in all.iter().filter(|e| e.session == session) {
        out.insert(e.message, match e.verdict {
            crate::core::feedback::Verdict::Up => "up".to_string(),
            crate::core::feedback::Verdict::Down => "down".to_string(),
        });
    }
    out.into_iter().collect()
}

/// What the feedback adds up to, as markdown. Read by a person, not by a model.
#[tauri::command]
pub fn feedback_report(state: State<'_, AppState>) -> String {
    crate::core::feedback::report(&crate::core::feedback::all(&state.data_dir)).render()
}

/// The reasons a downvote may carry, with the words the user sees.
#[tauri::command]
pub fn feedback_reasons() -> Vec<(String, String)> {
    crate::core::feedback::Reason::ALL
        .iter()
        .map(|r| (r.as_str().to_string(), r.label().to_string()))
        .collect()
}
