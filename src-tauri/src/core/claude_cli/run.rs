use super::*;

/// Run one turn through Claude Code and return what it answered.
///
/// The prompt goes on stdin: a conversation carries newlines, quotes and occasionally
/// something that looks like a flag, and argv is the wrong place for all three.
pub async fn run<F: FnMut(Update)>(model: &str, prompt: &str, cwd: &Path, mut on: F) -> Result<String, String> {
    let exe = bin().ok_or_else(|| {
        "The `claude` CLI isn't installed. Install Claude Code, run `claude` once to sign in, then try again.".to_string()
    })?;
    let mut cmd = Command::new(exe);
    cmd.args(argv(model))
        .current_dir(cwd)
        // Set by a Claude Code session in its own children. Inherited here — when this
        // app was itself started from one — it makes the CLI treat the turn as nested.
        .env_remove("CLAUDECODE")
        .env_remove("CLAUDE_CODE_ENTRYPOINT")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| format!("Couldn't start Claude Code: {e}"))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(prompt.as_bytes()).await.map_err(|e| format!("Couldn't send the prompt to Claude Code: {e}"))?;
        // `--print` reads to EOF before it starts, so this drop is load-bearing.
        drop(stdin);
    }

    // Drained on its own task: a full stderr pipe would block the CLI mid-turn while
    // we sat reading stdout — a hang, not an error.
    let stderr_task = child.stderr.take().map(|e| {
        tokio::spawn(async move {
            let mut buf = String::new();
            let mut lines = BufReader::new(e).lines();
            while let Ok(Some(l)) = lines.next_line().await {
                buf.push_str(&l);
                buf.push('\n');
            }
            buf
        })
    });

    let mut last_text = String::new();
    let mut failure: Option<String> = None;
    let mut done: Option<(bool, String)> = None;
    if let Some(stdout) = child.stdout.take() {
        let mut lines = BufReader::new(stdout).lines();
        while let Some(line) = lines.next_line().await.map_err(|e| format!("Lost Claude Code's output: {e}"))? {
            match parse(&line) {
                Some(Event::Said(said)) => {
                    for u in said {
                        if let Update::Text(t) = &u {
                            last_text = t.clone();
                        }
                        on(u);
                    }
                }
                Some(Event::Failure(why)) => failure = Some(why),
                Some(Event::Done { ok, text }) => done = Some((ok, text)),
                None => {}
            }
        }
    }

    let status = child.wait().await.map_err(|e| format!("Claude Code didn't finish: {e}"))?;
    let stderr = match stderr_task {
        Some(t) => t.await.unwrap_or_default(),
        None => String::new(),
    };

    match done {
        // `result` is the CLI's own statement of the answer; the last text block is
        // the fallback for a stream that ended without one.
        Some((true, text)) if !text.is_empty() => Ok(text),
        Some((true, _)) if !last_text.trim().is_empty() => Ok(last_text),
        Some((false, text)) => Err(explain(failure.as_deref().unwrap_or(&text), &stderr, model)),
        _ if status.success() && !last_text.trim().is_empty() => Ok(last_text),
        _ => Err(explain(failure.as_deref().unwrap_or(""), &stderr, model)),
    }
}

/// Turn a failure into something the user can act on.
pub(super) fn explain(said: &str, stderr: &str, model: &str) -> String {
    let all = format!("{said}\n{stderr}");
    let low = all.to_lowercase();
    if low.contains("failed to authenticate") || low.contains("oauth") || low.contains("not logged in") || low.contains("/login") {
        return "Claude Code isn't signed in any more. Run `claude` in a terminal, sign in with /login, then try again.".into();
    }
    if low.contains("model") && (low.contains("not found") || low.contains("invalid") || low.contains("not available")) {
        return format!("Claude Code can't run `{model}` on this sign-in. Pick another Claude Code model.");
    }
    if !said.trim().is_empty() {
        return format!("Claude Code: {}", said.trim());
    }
    let last = stderr
        .lines()
        .rev()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("Claude Code stopped without saying why.");
    format!("Claude Code: {last}")
}

/// Correction appended after the app's own instructions, which describe a loop that is
/// not running on this path. See `codex_cli::HANDOVER` for what happens without one.
pub(super) const HANDOVER: &str = "\
---

The instructions above were written for a different agent. You are running as a \
delegated agent with your own read-only tools (Read, Glob, Grep) in the user's \
working folder.

Do not ask for approval or permission — this turn has no interactive channel, so a \
question ends it with nothing done. Do the reading you need, then answer. Ignore any \
instruction above about this app's own tools, skills, or approval flow; use your own.";

/// Flatten a conversation into the single prompt `claude --print` takes.
pub fn flatten(system: &str, history: &[(String, String)], message: &str) -> String {
    crate::core::codex_cli::flatten_with(HANDOVER, system, history, message)
}
