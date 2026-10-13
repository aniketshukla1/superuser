//! Chat history

use super::*;

#[tauri::command]
pub fn list_chats(state: State<AppState>) -> Vec<crate::core::chats::ChatSummary> {
    crate::core::chats::list(&state.data_dir)
}

#[tauri::command]
pub fn get_chat(state: State<AppState>, id: String) -> Option<crate::core::chats::Conversation> {
    crate::core::chats::get(&state.data_dir, &id)
}

/// Persist a conversation (fast, no LLM). Returns the id (generated if new).
#[tauri::command]
pub fn save_chat(state: State<AppState>, id: Option<String>, title: Option<String>, messages: Vec<crate::core::chats::StoredMessage>) -> String {
    crate::core::chats::save(&state.data_dir, id, title, messages)
}

/// Summarize the conversation and write/refresh its searchable Chat memory.
#[tauri::command]
pub async fn index_chat(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let settings = state.settings.lock().unwrap().clone();
    let engine = crate::core::memory::MemoryEngine::new(state.memory.vault_path.clone());
    crate::core::chats::index(&state.data_dir, &engine, &settings, &id).await
}

#[tauri::command]
pub fn delete_chat(state: State<AppState>, id: String) {
    crate::core::chats::delete(&state.data_dir, &id, &state.memory)
}
