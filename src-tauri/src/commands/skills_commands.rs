//! Skills (SKILL.md, progressive disclosure)

use super::*;

/// Installed skills (name + description) for the Skills tab and session injection.
#[tauri::command]
pub fn skills_list(state: State<AppState>) -> Vec<crate::core::skills::SkillCard> {
    crate::core::skills::SkillStore::load(&state.data_dir).catalog()
}

/// One skill's full instructions (the `load_skill` payload).
#[tauri::command]
pub fn skill_get(state: State<AppState>, name: String) -> Option<crate::core::skills::Skill> {
    crate::core::skills::SkillStore::load(&state.data_dir).get(&name).cloned()
}
