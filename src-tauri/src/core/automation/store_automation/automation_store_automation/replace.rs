use super::*;

impl AutomationStore {
    /// Put a changed automation back, keeping what only the store knows — its run
    /// history, its counters, and the conversation it was asked for in.
    ///
    /// Takes the whole task rather than a patch because the caller has already decided
    /// what it should look like; merging two half-opinions here is how a field gets lost.
    pub fn replace(&mut self, id: &str, mut task: ScheduledTask) -> Result<ScheduledTask, String> {
        let before = self.get(id).cloned().ok_or_else(|| format!("unknown automation: {id}"))?;
        task.id = before.id;
        task.created_at = before.created_at;
        task.run_count = before.run_count;
        task.last_run = before.last_run;
        task.last_status = before.last_status;
        task.seen_runs_at = before.seen_runs_at;
        task.origin_session = before.origin_session.or(task.origin_session);
        task.updated_at = now();
        task.compute_next_run(now());
        if !task.enabled {
            task.next_run = None;
        }
        let slot = self.tasks.iter_mut().find(|t| t.id == id).ok_or_else(|| format!("unknown automation: {id}"))?;
        *slot = task.clone();
        self.save();
        Ok(task)
    }

    pub fn advance(&mut self, id: &str, after: f64) -> Result<(), String> {
        let t = self.get_mut(id).ok_or_else(|| format!("unknown automation: {id}"))?;
        t.compute_next_run(after);
        self.save();
        Ok(())
    }

    pub fn list(&self) -> Vec<ScheduledTask> {
        self.tasks.clone()
    }

    pub fn get(&self, id: &str) -> Option<&ScheduledTask> {
        self.tasks.iter().find(|t| t.id == id)
    }

    pub(in crate::core::automation) fn get_mut(&mut self, id: &str) -> Option<&mut ScheduledTask> {
        self.tasks.iter_mut().find(|t| t.id == id)
    }

    pub fn delete(&mut self, id: &str) -> bool {
        let before = self.tasks.len();
        self.tasks.retain(|t| t.id != id);
        self.runs.remove(id);
        let removed = self.tasks.len() != before;
        if removed {
            self.save();
        }
        removed
    }

    /// Repoint an automation at a different folder.
    ///
    /// Separate from `set_enabled` because it can fail on its own terms: a task
    /// pointed at a folder that isn't there runs every day and produces nothing,
    /// which is exactly the state this method exists to get out of — so it checks
    /// rather than accepting any string. `~` is expanded, since that is how a
    /// person writes a home path.
    pub fn set_workspace(&mut self, id: &str, workspace: &str) -> Result<String, String> {
        let expanded = if let Some(rest) = workspace.strip_prefix("~/") {
            dirs::home_dir().map(|h| h.join(rest)).ok_or("no home directory")?
        } else {
            std::path::PathBuf::from(workspace)
        };
        if !expanded.is_dir() {
            return Err(format!("{} is not a folder that exists", expanded.display()));
        }
        let path = expanded.to_string_lossy().to_string();
        let t = self.get_mut(id).ok_or_else(|| format!("unknown automation: {id}"))?;
        t.workspace = path.clone();
        t.updated_at = now();
        self.save();
        Ok(path)
    }

    /// Which model this automation runs on. `None` puts it back on whatever the app
    /// would pick, which is a real choice and not the same as "unset by accident" —
    /// so an empty string clears it rather than storing a model with no name.
    ///
    /// The field and the runner have both been here all along; nothing returned it and
    /// nothing could change it, so an automation's model could only ever be set once,
    /// by hand, through the create call. This is the half that was missing.
    pub fn set_model(&mut self, id: &str, model: Option<&str>) -> Result<Option<String>, String> {
        let chosen = model.map(str::trim).filter(|m| !m.is_empty()).map(str::to_string);
        let t = self.get_mut(id).ok_or_else(|| format!("unknown automation: {id}"))?;
        t.model = chosen.clone();
        t.updated_at = now();
        self.save();
        Ok(chosen)
    }

    /// Retitle, reword, or reschedule an automation.
    ///
    /// The detail view has offered all three since it was written and none of them
    /// worked: it PATCHes `{title, instructions, cron}`, the route accepted only
    /// `enabled` and `workspace`, and every save came back "nothing to update" — an
    /// editor that could open, accept your typing, and change nothing.
    ///
    /// A new cron recomputes `next_run` here rather than at the call site, because a
    /// schedule that has been changed and a next-run that has not is the shape of a
    /// task that fires at the old time and looks like it fires at the new one.
    pub fn edit(
        &mut self,
        id: &str,
        title: Option<&str>,
        instructions: Option<&str>,
        cron: Option<&str>,
    ) -> Result<(), String> {
        // Validate before touching anything: a half-applied edit is worse than a
        // refused one, and the caller can only report one outcome.
        let cron = cron.map(str::trim).filter(|c| !c.is_empty());
        if let Some(c) = cron {
            let probe = Schedule { kind: "cron".into(), cron: Some(c.to_string()), fire_at: None, timezone: local_tz() };
            if next_run_after(&probe, now()).is_none() {
                return Err(format!("`{c}` has no next run (e.g. `0 7 * * *` for 7am daily)"));
            }
        }
        let t = self.get_mut(id).ok_or_else(|| format!("unknown automation: {id}"))?;
        if let Some(v) = title.map(str::trim).filter(|v| !v.is_empty()) {
            t.title = v.to_string();
        }
        if let Some(v) = instructions.map(str::trim).filter(|v| !v.is_empty()) {
            t.instructions = v.to_string();
        }
        if let Some(c) = cron {
            t.schedule = Schedule { kind: "cron".into(), cron: Some(c.to_string()), fire_at: None, timezone: t.schedule.timezone.clone() };
            t.compute_next_run(now());
        }
        t.updated_at = now();
        self.save();
        Ok(())
    }

    pub fn set_enabled(&mut self, id: &str, enabled: bool) -> Result<(), String> {
        if enabled && self.get(id).is_some_and(|t| !t.enabled) && self.enabled_count() >= MAX_ENABLED {
            return Err(format!(
                "There are already {MAX_ENABLED} automations switched on, which is as many as this machine keeps. Switch one off first."
            ));
        }
        let t = self.get_mut(id).ok_or_else(|| format!("unknown automation: {id}"))?;
        t.enabled = enabled;
        if enabled && t.next_run.is_none() {
            t.compute_next_run(now());
        }
        t.updated_at = now();
        self.save();
        Ok(())
    }

    pub fn add_rule(&mut self, id: &str, tool: &str, target: &str) -> Result<bool, String> {
        let t = self.get_mut(id).ok_or_else(|| format!("unknown automation: {id}"))?;
        let added = t.add_rule(tool, target);
        if added {
            self.save();
        }
        Ok(added)
    }

    pub fn revoke_rule(&mut self, id: &str, entry: &str) -> Result<bool, String> {
        let t = self.get_mut(id).ok_or_else(|| format!("unknown automation: {id}"))?;
        let revoked = t.revoke_rule(entry);
        if revoked {
            self.save();
        }
        Ok(revoked)
    }

    /// Whether `message` is the instruction of the manual run still going in `session_id`.
    pub fn opens_manual_run(&self, session_id: &str, message: &str) -> bool {
        self.runs.values().flatten().any(|r| {
            r.session_id == session_id
                && r.trigger == "manual"
                && r.status == "running"
                && self.get(&r.task_id).is_some_and(|t| t.instructions.trim() == message.trim())
        })
    }
}
