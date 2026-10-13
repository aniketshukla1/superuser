use super::*;

/// The brain half. `scratch_vault` is `None` on any instance that is not provably
/// disposable, and a brain task then fails with a reason rather than running against
/// whatever memories happen to be lying around — which would score the operator's vault,
/// not the harness.
pub(super) fn seed_brain(settings: &Value, task: &Task, marker: &str) -> Result<(), String> {
    match (scratch_vault(settings), task.vault.is_empty()) {
        // Cleared even when this task seeds nothing, so a later task cannot inherit an
        // earlier one's memories.
        (Some(vault), _) => seed_vault(&vault, task, marker),
        (None, true) => Ok(()),
        (None, false) => {
            Err("brain task needs a scratch brain: run the app and this driver with the same SUPERUSER_DATA_DIR".into())
        }
    }
}
