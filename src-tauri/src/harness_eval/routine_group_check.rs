//! The routine group's check

use super::*;

/// The tools that manage automations rather than doing work.
pub(super) const SCHEDULING_TOOLS: &[&str] = &["list_automations", "schedule_task", "update_automation", "delete_automation", "watch_for"];

/// A firing did the work it was given, and did not spend the turn on the automation
/// itself. Both halves matter: the known failure looked like a success because the model
/// listed its routines, changed nothing, and said so cheerfully.
pub(super) fn did_the_work_not_the_scheduling(r: &TurnRecord) -> Result<(), String> {
    if let Some(c) = r.calls.iter().find(|c| SCHEDULING_TOOLS.contains(&c.name.as_str())) {
        return Err(format!("spent the firing on the automation instead of the work: {}", c.name));
    }
    let landed = r.files.values().any(|body| !r.marker.is_empty() && body.contains(&r.marker));
    if !landed {
        let wrote: Vec<&str> = r.calls.iter().map(|c| c.name.as_str()).collect();
        return Err(format!("the work didn't land in any file; called {wrote:?}"));
    }
    Ok(())
}
