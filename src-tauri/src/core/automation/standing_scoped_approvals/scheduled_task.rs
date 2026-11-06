use super::super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTask {
    pub title: String,
    pub instructions: String,
    pub schedule: Schedule,
    pub workspace: String,
    #[serde(default = "default_agent")]
    pub agent: String,
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default = "tru")]
    pub notify_on_completion: bool,
    #[serde(default)]
    pub notify_target: Option<String>, // extra messaging target, e.g. "telegram:123"
    #[serde(default)]
    pub always_allowed_tools: Vec<String>,
    #[serde(default = "tru")]
    pub enabled: bool,
    #[serde(default = "now")]
    pub created_at: f64,
    #[serde(default = "now")]
    pub updated_at: f64,
    #[serde(default)]
    pub next_run: Option<f64>,
    #[serde(default)]
    pub last_run: Option<f64>,
    #[serde(default)]
    pub last_status: Option<String>,
    #[serde(default)]
    pub run_count: u32,
    #[serde(default)]
    pub max_runs: Option<u32>,
    #[serde(default)]
    pub seen_runs_at: f64,
    /// The chat this automation was set up from, if any. A scheduled run executes
    /// in its own isolated session, but its *answer* is written back here so the
    /// user can follow up on it in the conversation where they asked for it —
    /// otherwise a daily brief lands somewhere they never look and can't reply to.
    #[serde(default)]
    pub origin_session: Option<String>,
}

impl ScheduledTask {
    pub fn new(title: String, instructions: String, schedule: Schedule, workspace: String, agent: Option<String>, model: Option<String>) -> Self {
        let t = now();
        ScheduledTask {
            id: gen_id("task"),
            title,
            instructions,
            schedule,
            workspace,
            agent: agent.unwrap_or_else(default_agent),
            model,
            notify_on_completion: true,
            notify_target: None,
            always_allowed_tools: Vec::new(),
            enabled: true,
            created_at: t,
            updated_at: t,
            next_run: None,
            last_run: None,
            last_status: None,
            run_count: 0,
            max_runs: None,
            seen_runs_at: 0.0,
            origin_session: None,
        }
    }

    /// The task's own conversation thread id.
    pub fn task_session_id(&self) -> String {
        format!("__task__{}", self.id)
    }

    pub fn human_schedule(&self) -> String {
        self.schedule.human()
    }

    /// Set `next_run` to the next fire strictly after `after`.
    pub fn compute_next_run(&mut self, after: f64) {
        self.next_run = next_run_after(&self.schedule, after);
    }

    /// The entries that name a target, as {tool: [targets]}, which is what the permission engine reads.
    pub fn standing_rules(&self) -> HashMap<String, Vec<String>> {
        let mut out: HashMap<String, Vec<String>> = HashMap::new();
        for entry in &self.always_allowed_tools {
            let (tool, target) = rule_parts(entry);
            if let Some(t) = target {
                out.entry(tool).or_default().push(t);
            }
        }
        out
    }

    pub fn add_rule(&mut self, tool: &str, target: &str) -> bool {
        let entry = rule_entry(tool, Some(target));
        if tool.is_empty() || target.is_empty() || self.always_allowed_tools.contains(&entry) {
            return false;
        }
        self.always_allowed_tools.push(entry);
        true
    }

    pub fn revoke_rule(&mut self, entry: &str) -> bool {
        if let Some(pos) = self.always_allowed_tools.iter().position(|e| e == entry) {
            self.always_allowed_tools.remove(pos);
            true
        } else {
            false
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRun {
    pub task_id: String,
    #[serde(default)]
    pub run_id: String,
    #[serde(default = "now")]
    pub started_at: f64,
    #[serde(default)]
    pub finished_at: Option<f64>,
    #[serde(default = "status_running")]
    pub status: String, // running | ok | error | skipped
    #[serde(default)]
    pub result_text: Option<String>,
    #[serde(default)]
    pub artifacts: Vec<String>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default = "trigger_schedule")]
    pub trigger: String, // schedule | manual | catchup
    #[serde(default)]
    pub session_id: String,
}

impl TaskRun {
    pub fn new(task_id: String, trigger: &str) -> Self {
        let run_id = gen_id("run");
        let session_id = format!("__run__{run_id}");
        TaskRun {
            task_id,
            run_id,
            started_at: now(),
            finished_at: None,
            status: "running".into(),
            result_text: None,
            artifacts: Vec::new(),
            error: None,
            trigger: trigger.to_string(),
            session_id,
        }
    }
}
