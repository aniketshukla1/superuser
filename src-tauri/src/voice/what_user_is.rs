//! What the user is looking at


/// Tell the running session which documents are open in the panel.
///
/// Called by the panel whenever its document tabs change, so a sentence like
/// "add a row to this" resolves to the file in front of the user rather than to
/// the agent's best guess.
#[tauri::command]
pub fn set_open_documents(
    session_id: String,
    open: Vec<String>,
    active: Option<String>,
) {
    crate::core::opendocs::set(
        &session_id,
        crate::core::opendocs::OpenDocs { open, active },
    );
}

#[tauri::command]
pub fn open_documents(session_id: String) -> crate::core::opendocs::OpenDocs {
    crate::core::opendocs::get(&session_id)
}
