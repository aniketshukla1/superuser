use super::*;

pub(super) fn run_shell_tools(root: &Path, todo: &Mutex<Vec<TodoItem>>, name: &str, args: &Value) -> Option<String> {
    let root_s = root.to_string_lossy().to_string();

    let out = match name {
        "run_shell" => {
            let cmd = args["command"].as_str().unwrap_or("").trim();
            if cmd.is_empty() {
                return Some("error: empty command".into());
            }
            // The floor, and it has to be HERE rather than inside either runner.
            // Both paths below execute; only the fallback carried a check, so with
            // the Terminal panel open a command went to the pty ungated — the one
            // state in which the user is watching, and the one in which nothing was
            // stopping it.
            if let Some(reason) = crate::core::cmdgate::hard_deny(cmd) {
                return Some(format!(
                    "blocked: {reason}. Nothing ran. If you genuinely need this, say what you are \
                     trying to achieve and let the user run it themselves."
                ));
            }
            // Prefer the shared terminal: when the Terminal tab is open the command runs
            // in the shell the user is watching, so they see it happen instead of reading
            // about it afterwards. With no terminal open (or one already busy with another
            // agent command) this falls back to a detached process, so the tool never
            // depends on a panel being visible.
            let (code, output) = match crate::core::terminal::run_captured(cmd, 120) {
                Some(captured) => captured,
                None => coder::run_capture(&root_s, cmd, 120, 12000),
            };
            format!("exit {code}\n{output}")
        }
        "git_status" => serde_json::to_string_pretty(&coder::git(&root_s)).unwrap_or_default(),
        "git_diff" => {
            let (unstaged, staged) = coder::git_diff(&root_s, args["path"].as_str().unwrap_or(""), false);
            let mut s = unstaged;
            if !staged.trim().is_empty() {
                s.push_str("\n--- staged ---\n");
                s.push_str(&staged);
            }
            if s.trim().is_empty() { "(no changes)".into() } else { s }
        }
        "git_log" => serde_json::to_string_pretty(&coder::git_log(&root_s)).unwrap_or_default(),
        "todo_write" => {
            let items = normalize_todos(&args["todos"]);
            let n = items.len();
            *todo.lock().unwrap() = items;
            format!("updated task list ({n} items)")
        }
        "propose_plan" => format!("Plan proposed for your approval:\n{}", args["plan"].as_str().unwrap_or("")),
        _ => unreachable!("routed here only for its own tools"),
    };
    Some(out)
}
