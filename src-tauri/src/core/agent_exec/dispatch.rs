use super::*;

mod process_metrics_tools;
mod memory_knowledge_tools;
mod automation_tools;
mod skill_web_tools;
mod connector_message_tools;

use process_metrics_tools::*;
use memory_knowledge_tools::*;
use automation_tools::*;
use skill_web_tools::*;
use connector_message_tools::*;

/// Execute any catalog tool (local first, then the async ones: recall/web).
pub async fn dispatch(ctx: &ToolCtx<'_>, name: &str, args: &Value) -> String {
    if let Some(out) = dispatch_toolchain(ctx, name, args).await {
        return out;
    }
    if let Some(out) = dispatch_board(ctx, name, args) {
        return out;
    }
    // Before `dispatch_local`, which has no `ctx` and so cannot see the mode, the
    // taint, or the session's grants.
    if name == "run_shell" {
        if let Some(refusal) = gate_run_shell(ctx, args).await {
            return refusal;
        }
    }
    if let Some(out) = dispatch_local(&ctx.root, &ctx.todo, name, args) {
        return out;
    }
    match name {
        "process_start" | "process_read" | "process_send" | "process_stop" | "expense_summary" |
        "metrics_find" | "metrics_query" | "metrics_range" | "metrics_compare" | "metrics_alerts" |
        "perf_report" | "activity_since" | "recall_output" | "ask_user" | "ask_form" |
        "report_refusal" => process_metrics_tools(ctx, name, args).await,
        "recall_memory" | "knowledge_write" | "knowledge_search" | "knowledge_read" => memory_knowledge_tools(ctx, name, args).await,
        "watch_for" | "schedule_task" | "list_automations" | "update_automation" |
        "delete_automation" => automation_tools(ctx, name, args).await,
        name if crate::core::agent::is_browser_tool(name) => browser_tool(ctx, name, args).await,
        "list_skills" | "read_skill" | "list_skill_tools" | "list_personas" | "read_persona" |
        "list_persona_skills" | "save_skill" | "save_persona" | "web_search" | "browser_open" |
        "web_fetch" | "list_connectors" | "connector_search" => skill_web_tools(ctx, name, args).await,
        "connector_changes" | "connector_sync" | "send_message" | "computer_use" |
        "spawn_subagent" | "load_skill" => connector_message_tools(ctx, name, args).await,
        other => format!("unknown tool: {other}"),
    }
}
