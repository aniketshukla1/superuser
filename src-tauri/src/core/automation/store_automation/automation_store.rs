use super::super::*;

/// JSON-file-backed store: tasks in `automations.json`, runs in `automation-runs.json`.
pub struct AutomationStore {
    pub(in crate::core::automation) data_dir: PathBuf,
    pub(in crate::core::automation) tasks: Vec<ScheduledTask>,
    pub(in crate::core::automation) runs: HashMap<String, Vec<TaskRun>>,
}
