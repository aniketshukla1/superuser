use super::*;

impl AutomationStore {
    pub fn load(data_dir: &Path) -> Self {
        let tasks = fs::read_to_string(data_dir.join("automations.json"))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        let runs = fs::read_to_string(data_dir.join("automation-runs.json"))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        let mut store = Self { data_dir: data_dir.to_path_buf(), tasks, runs };
        // An enabled task with no next fire never fires, and says nothing about it:
        // `due()` reads `next_run <= now` and `None` is not `<=` anything, so it sits
        // in the list looking scheduled and armed forever. Only `create` and
        // `set_enabled` ever computed one, so a task that lost its next_run any other
        // way — a hand-edited file, a partial write, a schedule changed elsewhere —
        // was quietly dead. Heal it on the way in, where every path goes.
        let healed: Vec<String> = store
            .tasks
            .iter_mut()
            .filter(|t| t.enabled && t.next_run.is_none())
            .map(|t| {
                t.compute_next_run(now());
                t.id.clone()
            })
            .collect();
        if !healed.is_empty() {
            store.save();
        }
        store
    }

    pub(in crate::core::automation) fn save(&self) {
        // Atomic. The scheduler saves these every ~30s from a background task
        // while the API reads them for the Automations page, and a torn read comes
        // back as an empty list — which the next load-mutate-save would persist,
        // taking every scheduled task with it. See `core::store`.
        let _ = crate::core::store::write_json_atomic(&self.data_dir.join("automations.json"), &self.tasks);
        let _ = crate::core::store::write_json_atomic(&self.data_dir.join("automation-runs.json"), &self.runs);
    }

    pub fn create(&mut self, mut task: ScheduledTask) -> ScheduledTask {
        // Compute the first fire unless the caller already set one.
        if task.next_run.is_none() {
            task.compute_next_run(now());
        }
        self.tasks.push(task.clone());
        self.save();
        task
    }

    /// The same, refusing what should not exist: a schedule the app cannot keep, one that
    /// fires too often, or one more than this machine allows to be switched on.
    ///
    /// Sentences, not codes: every caller hands these to a model or to a person.
    pub fn create_checked(&mut self, task: ScheduledTask) -> Result<ScheduledTask, String> {
        if let Some(problem) = schedule_problem(&task.schedule) {
            return Err(problem);
        }
        if task.enabled && self.enabled_count() >= MAX_ENABLED {
            return Err(format!(
                "There are already {MAX_ENABLED} automations switched on, which is as many as this machine keeps. Switch one off first."
            ));
        }
        Ok(self.create(task))
    }

    /// Enabled tasks whose next fire is due (`next_run <= now`) and that haven't hit
    /// their run cap — the set the scheduler should fire.
    pub fn due(&self, now: f64) -> Vec<ScheduledTask> {
        self.tasks
            .iter()
            .filter(|t| {
                t.enabled
                    && t.next_run.map(|n| n <= now).unwrap_or(false)
                    && t.max_runs.map(|m| t.run_count < m).unwrap_or(true)
            })
            .cloned()
            .collect()
    }

    /// Recompute a task's next fire (called after it runs).
    /// How many automations are switched on right now.
    pub fn enabled_count(&self) -> usize {
        self.tasks.iter().filter(|t| t.enabled).count()
    }

    /// Firings that are due but too late to be worth running, advanced past silently.
    ///
    /// Called before `due()` so a laptop that has been shut for a week wakes up to today's
    /// work rather than to a week of it. The skip is written down — a run that did not
    /// happen is a fact about the automation, and the page shows it as `skipped`.
    pub fn skip_missed(&mut self, now: f64) -> Vec<String> {
        let stale: Vec<String> = self
            .tasks
            .iter()
            .filter(|t| t.enabled && t.next_run.is_some_and(|n| now - n > GRACE_SECS))
            .map(|t| t.id.clone())
            .collect();
        for id in &stale {
            if let Some(t) = self.get_mut(id) {
                t.last_status = Some("skipped".into());
                t.compute_next_run(now);
            }
            if let Ok(run) = self.start_run(id, "schedule") {
                let _ = self.finish_run(id, &run.run_id, "skipped", None, vec![], Some("missed its window".into()));
            }
        }
        if !stale.is_empty() {
            self.save();
        }
        stale
    }

    /// Runs that were started and never finished — the app was closed, or it crashed.
    /// Written off rather than left in flight, where they look like work still happening.
    pub fn reap_abandoned(&mut self, now: f64) -> usize {
        let mut reaped = 0;
        let ids: Vec<String> = self.runs.keys().cloned().collect();
        for id in ids {
            let stale: Vec<String> = self
                .runs
                .get(&id)
                .map(|runs| {
                    runs.iter()
                        .filter(|r| r.status == "running" && now - r.started_at > ABANDONED_SECS)
                        .map(|r| r.run_id.clone())
                        .collect()
                })
                .unwrap_or_default();
            for run_id in stale {
                let _ = self.finish_run(&id, &run_id, "skipped", None, vec![], Some("the app stopped before it finished".into()));
                reaped += 1;
            }
        }
        reaped
    }

    /// How many times in a row this automation's firings have failed.
    ///
    /// Skipped runs are not failures and do not break the streak: a laptop that was shut
    /// says nothing about whether the work works.
    pub fn consecutive_failures(&self, id: &str) -> u32 {
        let mut n = 0;
        for run in self.runs.get(id).map(|r| r.iter().rev().collect::<Vec<_>>()).unwrap_or_default() {
            match run.status.as_str() {
                "skipped" | "running" => continue,
                "error" => n += 1,
                _ => break,
            }
            if n >= FATIGUE_LIMIT * 2 {
                break;
            }
        }
        n
    }

    /// What to say after a firing failed, and whether this is the one that switches it
    /// off — the fatigue rule.
    ///
    /// One message on the first failure after a success, then silence until the tenth,
    /// which switches it off and says so. The quiet middle is the point: an automation
    /// that fails every morning must not send a message every morning.
    pub fn fatigue(&mut self, id: &str, reason: &str) -> Option<String> {
        let failures = self.consecutive_failures(id);
        if failures >= FATIGUE_LIMIT {
            if let Some(t) = self.get_mut(id) {
                t.enabled = false;
                t.next_run = None;
            }
            self.save();
            return Some(format!(
                "I've switched this automation off after {FATIGUE_LIMIT} failures in a row. Once whatever it depends on is fixed, ask me and I'll turn it back on."
            ));
        }
        (failures == 1).then(|| format!("This automation failed: {}", reason.chars().take(160).collect::<String>()))
    }
}
