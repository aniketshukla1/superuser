use super::*;

mod read_file_tools;
mod slides_edit_tools;
mod table_tools;
mod chart_tools;
mod sheet_read_tools;
mod run_shell_tools;

use read_file_tools::*;
use slides_edit_tools::*;
use table_tools::*;
use chart_tools::*;
use sheet_read_tools::*;
use run_shell_tools::*;

/// Execute a *local* tool (files/search/shell/git/todo/plan/ask) synchronously.
/// Returns None if `name` isn't a local tool (dispatch then tries the async ones).
pub fn dispatch_local(root: &Path, todo: &Mutex<Vec<TodoItem>>, name: &str, args: &Value) -> Option<String> {
    match name {
        "read_file" | "write_file" | "doc_read" | "doc_edit" | "slides_read" => read_file_tools(root, todo, name, args),
        "slides_edit" | "doc_write" | "slides_write" | "ui_component" | "card" => slides_edit_tools(root, todo, name, args),
        "table" => table_tools(root, todo, name, args),
        "chart" => chart_tools(root, todo, name, args),
        "sheet_read" | "sheet_new" | "sheet_write" | "sheet_sources" | "sheet_profile" |
        "edit_file" | "list_dir" | "grep" => sheet_read_tools(root, todo, name, args),
        "run_shell" | "git_status" | "git_diff" | "git_log" | "todo_write" | "propose_plan" => run_shell_tools(root, todo, name, args),
        // `ask_user` is deliberately NOT here. It used to be, returning the string
        // "[awaiting user] <question>" — which is not a question being asked, it is a
        // question being narrated to nobody while the loop carries on. It needs to
        // wait for a real answer, so it lives in the async `dispatch` below.
        _ => None,
    }
}
