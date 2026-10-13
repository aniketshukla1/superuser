use super::*;

impl AutomationStore {
    /// Begin a run (records it against the task's run log).
    pub fn start_run(&mut self, task_id: &str, trigger: &str) -> Result<TaskRun, String> {
        if self.get(task_id).is_none() {
            return Err(format!("unknown automation: {task_id}"));
        }
        let run = TaskRun::new(task_id.to_string(), trigger);
        self.runs.entry(task_id.to_string()).or_default().push(run.clone());
        self.save();
        Ok(run)
    }

    /// Finalize a run and roll its outcome onto the task (last_run/last_status/run_count).
    pub fn finish_run(&mut self, task_id: &str, run_id: &str, status: &str, result: Option<String>, artifacts: Vec<String>, error: Option<String>) -> Result<(), String> {
        let runs = self.runs.get_mut(task_id).ok_or_else(|| format!("no runs for {task_id}"))?;
        let run = runs.iter_mut().find(|r| r.run_id == run_id).ok_or_else(|| format!("unknown run: {run_id}"))?;
        run.finished_at = Some(now());
        run.status = status.to_string();
        run.result_text = result;
        run.artifacts = artifacts;
        run.error = error;
        if let Some(t) = self.get_mut(task_id) {
            t.last_run = Some(now());
            t.last_status = Some(status.to_string());
            t.run_count += 1;
        }
        self.save();
        Ok(())
    }

    pub fn list_runs(&self, task_id: &str) -> Vec<TaskRun> {
        self.runs.get(task_id).cloned().unwrap_or_default()
    }

    /// Runs this task has finished that the user hasn't looked at, and whether the
    /// newest of them failed.
    ///
    /// A run is "seen" once its detail page has been opened, not once it exists —
    /// an automation's whole point is that it happens while you are elsewhere, so
    /// the only useful unread mark is one you clear by actually reading it.
    /// Unfinished runs don't count: a run still going is not news yet.
    pub fn unseen(&self, task_id: &str) -> (usize, bool) {
        let since = self.get(task_id).map(|t| t.seen_runs_at).unwrap_or(0.0);
        let mut fresh: Vec<&TaskRun> = self
            .runs
            .get(task_id)
            .map(|r| r.iter().filter(|r| r.finished_at.map(|f| f > since).unwrap_or(false)).collect())
            .unwrap_or_default();
        fresh.sort_by(|a, b| a.started_at.total_cmp(&b.started_at));
        let failed = fresh.last().map(|r| r.status == "error").unwrap_or(false);
        (fresh.len(), failed)
    }

    /// Mark everything up to now as read.
    pub fn mark_seen(&mut self, task_id: &str) -> Result<(), String> {
        let t = self.get_mut(task_id).ok_or_else(|| format!("unknown automation: {task_id}"))?;
        t.seen_runs_at = now();
        self.save();
        Ok(())
    }
}
