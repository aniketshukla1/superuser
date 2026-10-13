use super::*;

/// Persist the composer's model or mode, and say which one changed.
pub(super) async fn set_model_or_mode(tx: &mut WsSink, st: &crate::core::sessions::SessionStore, id: &String, frame: &Value) {
    // Persist the composer's choice so the next turn (and a reload) uses it.
    //
    // Both frames land here because both write one field of the same
    // record — but they must not REPORT the same thing. The reply used to
    // be `model_changed` either way, and the GUI writes "Model switched"
    // into the transcript when it sees one, so changing your permission
    // mode left a durable line in the conversation claiming the model had
    // changed. It had not. A transcript that misreports what happened is
    // worse than one that says nothing.
    if let Some(mut s) = st.get(&id) {
        let mut model_changed = false;
        let mut mode_changed = false;
        if let Some(m) = frame["model"].as_str().filter(|m| !m.is_empty()) {
            model_changed = s.model != m;
            s.model = m.to_string();
        }
        if let Some(m) = frame["mode"].as_str().filter(|m| !m.is_empty()) {
            mode_changed = s.mode != m;
            s.mode = m.to_string();
        }
        st.save(&s);
        // `mode_changed` carries no transcript notice: the mode is shown
        // in the composer, where the user just set it, and a line per
        // toggle would bury the conversation in its own settings history.
        let kind = if model_changed { "model_changed" } else { "mode_changed" };
        crate::trace!(
            "chat",
            "set session={id} kind={kind} model={} mode={} (model_changed={model_changed} mode_changed={mode_changed})",
            s.model,
            s.mode
        );
        let _ = send(tx, json!({ "type": kind, "data": { "model": s.model, "mode": s.mode } })).await;
    }
}
