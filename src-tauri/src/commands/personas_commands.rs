//! Personas (assistants)

use super::*;

/// Every installed persona + lifecycle state (for the Personas settings panel).
#[tauri::command]
pub fn personas_list(state: State<AppState>) -> Vec<crate::core::personas::PersonaListItem> {
    crate::core::personas::PersonaRegistry::load(&state.data_dir).list_all()
}

/// The new-session assistant picker: enabled + surfaced personas.
#[tauri::command]
pub fn personas_picker(state: State<AppState>) -> Vec<crate::core::personas::PersonaPickerItem> {
    crate::core::personas::PersonaRegistry::load(&state.data_dir).sidebar()
}

/// Resolve one persona's full manifest (prompt, tools, recommends).
#[tauri::command]
pub fn persona_get(state: State<AppState>, id: String) -> Option<crate::core::personas::PersonaManifest> {
    crate::core::personas::PersonaRegistry::load(&state.data_dir).manifest(&id).cloned()
}

#[tauri::command]
pub fn persona_set_enabled(state: State<AppState>, id: String, enabled: bool) -> Result<(), String> {
    crate::core::personas::PersonaRegistry::load(&state.data_dir).set_enabled(&id, enabled)
}

#[tauri::command]
pub fn persona_set_surfaced(state: State<AppState>, id: String, surfaced: bool) -> Result<(), String> {
    crate::core::personas::PersonaRegistry::load(&state.data_dir).set_surfaced(&id, surfaced)
}

#[tauri::command]
pub fn persona_set_default(state: State<AppState>, id: String) -> Result<(), String> {
    crate::core::personas::PersonaRegistry::load(&state.data_dir).set_default(&id)
}

/// The capability catalog (what any assistant can be granted) — for the UI.
#[tauri::command]
pub fn agent_catalog() -> Vec<crate::core::agent::CapabilityInfo> {
    crate::core::agent::catalog_info()
}
