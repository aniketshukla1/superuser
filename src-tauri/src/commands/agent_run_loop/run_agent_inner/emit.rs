use super::*;

/// One step of the turn, sent to the GUI and to the session socket if one is attached.
pub(super) fn turn_event(app: &tauri::AppHandle, sink: &Option<EventSink>, kind: &str, text: String) {
    use tauri::Emitter;
    let payload = serde_json::json!({ "type": kind, "text": text });
    let _ = app.emit("agent-run", payload.clone());
    // The session socket (if one is attached) renders the same steps live.
    if let Some(tx) = sink {
        let _ = tx.send(payload);
    }
}
