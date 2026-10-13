use super::*;

/// Tools no boundary applies to.
///
/// Each of these changes nothing and reaches nothing: asking the user a question,
/// keeping the visible task list, proposing a plan, reading back this run's own
/// truncated output, loading a skill's instructions, and saying that it declined
/// something. Control events stay out of policy for the same reason — a rule must
/// never lock a person out of their own browser mid-login. A rule
/// broad enough to catch these would leave an assistant unable to even explain that
/// it was stopped.
pub const UNGOVERNED: &[&str] = &[
    "ask_user", "ask_form", "todo_write", "propose_plan", "recall_output", "load_skill", "report_refusal", "find_tools",
    // Asking the user to take the browser, or for one value: control events, kept out of
    // policy so a rule can never lock a person out of their own browser at a
    // sign-in. They write their own `computer.*` rows.
    "browser_request_help", "browser_request_secret",
];

/// Who is acting, in which conversation, started by what.
#[derive(Debug, Clone)]
pub struct Facts {
    pub persona: String,
    pub session: String,
    pub initiator: Initiator,
}

/// A connected app's tool, as the MCP branch knows it.
#[derive(Debug, Clone, Default)]
pub struct McpCall {
    pub server: String,
    pub tool: String,
    /// `read` or `write`, from the effective risk. Anything not positively a read is
    /// a write.
    pub effect: String,
}

/// Every catalogue tool's risk class, built once.
pub(super) fn catalogue_risk() -> &'static HashMap<String, RiskClass> {
    static RISK: OnceLock<HashMap<String, RiskClass>> = OnceLock::new();
    RISK.get_or_init(|| {
        let mut m: HashMap<String, RiskClass> = crate::core::agent::every_tool_risk().into_iter().collect();
        m.insert("run_plan".into(), RiskClass::Exec);
        m.insert("load_skill".into(), RiskClass::Read);
        m
    })
}

pub(super) fn risk_name(r: RiskClass) -> &'static str {
    match r {
        RiskClass::Read => "read",
        RiskClass::WriteLocal => "write_local",
        RiskClass::Exec => "exec",
        RiskClass::External => "external",
    }
}

/// What an action does.
///
/// Derived centrally, here, and never at a call site: a tool that forgot to say what
/// it does would otherwise be the one no rule could name. The common intents
/// (`read_file`, `list_files`, `write_file`, `run_command`, `navigate`, `read_tool`,
/// `write_tool`) are joined by Superuser's own: `search`, `send`, `schedule`, `remember`,
/// `delegate`, `use_computer`, `show`, `track` and `read`.
pub fn intent_of(name: &str, args: &Value, mcp: Option<&McpCall>) -> &'static str {
    if let Some(m) = mcp {
        return if m.effect == "read" { "read_tool" } else { "write_tool" };
    }
    let has_path = || args["path"].as_str().is_some_and(|p| !p.trim().is_empty());
    match name {
        "read_file" | "doc_read" | "slides_read" | "sheet_read" | "sheet_profile" | "sheet_sources" | "grep" | "git_status"
        | "git_diff" | "git_log" => "read_file",
        "list_dir" => "list_files",
        "write_file" | "edit_file" | "doc_write" | "doc_edit" | "slides_write" | "slides_edit" | "sheet_new" | "sheet_write" => {
            "write_file"
        }
        // Drawing something in the conversation touches nothing — unless it is also
        // saved, and then it is a file like any other.
        "ui_component" => "show",
        "chart" | "table" | "card" => {
            if has_path() {
                "write_file"
            } else {
                "show"
            }
        }
        // `request_tool` ends in an install command the user approves.
        "run_shell" | "process_start" | "process_send" | "process_stop" | "request_tool" => "run_command",
        "web_fetch" | "browser_open" | "browser_navigate" => "navigate",
        // The browser intents. A button is pressed by a click, by Enter or Space on
        // it, or by typing that ends in Enter — a form has three doors, and a rule about
        // pressing things has to see all three as the same act.
        "browser_click" => "activate",
        "browser_type" => {
            if args["submit"].as_bool().unwrap_or(false) {
                "activate"
            } else {
                "type"
            }
        }
        "browser_key" => {
            if crate::core::agentbrowser::keys::activates(args["key"].as_str().unwrap_or("")) {
                "activate"
            } else {
                "type"
            }
        }
        "browser_read" | "browser_snapshot" | "browser_scroll" | "browser_console" | "browser_screenshot" => "read",
        "metrics_find" | "metrics_query" | "metrics_range" | "metrics_compare" | "metrics_alerts" | "perf_report" => "read",
        "web_search" => "search",
        "send_message" => "send",
        "schedule_task" | "watch_for" | "update_automation" | "delete_automation" => "schedule",
        "list_automations" => "read",
        "knowledge_write" => "remember",
        "spawn_subagent" | "ask_coworker" => "delegate",
        // Writing a skill or an assistant: a file in the app's own directory that every
        // later run reads, which is a different act from writing a file in the workspace.
        "save_skill" | "save_persona" => "author",
        "list_skills" | "read_skill" | "list_skill_tools" | "list_personas" | "read_persona" | "list_persona_skills" => "read",
        "computer_use" => "use_computer",
        // Pulls another system's content onto this machine: a call to that system.
        "connector_sync" => "read_tool",
        "board_add" | "board_assign" | "board_move" | "board_note" | "board_review" | "record_coverage" | "coverage_note"
        | "check_tool" => "track",
        "recall_memory" | "knowledge_search" | "knowledge_read" | "connector_search" | "connector_changes" | "list_connectors"
        | "board_list" | "process_read" | "expense_summary" | "activity_since" => "read",
        _ => "",
    }
}
