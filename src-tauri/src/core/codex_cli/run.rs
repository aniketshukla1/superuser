use super::*;

/// What `codex exec` is told, beyond the prompt.
pub(super) const SANDBOX: &str = "read-only";

/// The exact command line handed to `codex exec`.
///
/// Split out to be testable. These flags are an unversioned CLI contract with no
/// compiler behind it: a missing one doesn't fail to build, it fails every turn at
/// runtime with a message about something else — which is exactly how
/// `--skip-git-repo-check` was missed the first time.
pub(super) fn argv(model: &str, cwd: &Path) -> Vec<String> {
    vec![
        "exec".into(),
        "--sandbox".into(),
        SANDBOX.into(),
        // Codex refuses to start outside a Git repository unless told otherwise,
        // and the directory it is handed here is a session scratch dir that is
        // not one. Without this, every turn fails with "Not inside a trusted
        // directory" regardless of what was asked.
        //
        // That check guards against an agent editing files somewhere the user did
        // not expect — reasoning about writes. `SANDBOX` is read-only, so this
        // waives a confirmation about changing things for a process that cannot
        // change anything.
        "--skip-git-repo-check".into(),
        // JSONL on stdout, one event per line, as they happen. Without it the CLI
        // prints prose and only at the end, so a turn that takes a minute shows
        // nothing for a minute and then everything at once.
        "--json".into(),
        // Codex asks before running a command, and on this path there is nobody to
        // answer: stdin closed after the prompt. It would ask, wait, and end the
        // turn having done nothing — which reads as the model refusing rather than
        // as a dialogue nobody could hear.
        //
        // This waives the *prompt*, not the sandbox. `--sandbox read-only` still
        // bounds what any command can do. The two flags the CLI offers for this
        // both go further than we want — `--approve-for-me` escalates to
        // workspace-write, and the bypass flag drops the sandbox entirely — so the
        // policy is set directly instead.
        "-c".into(),
        "approval_policy=\"never\"".into(),
        "-C".into(),
        cwd.to_string_lossy().into_owned(),
        "-m".into(),
        model.into(),
        // The prompt arrives on stdin; see `run`.
        "-".into(),
    ]
}

/// Something worth showing the user while Codex is still working.
///
/// Deliberately not the whole event vocabulary: `thread.started`, `item.started`
/// and the token counts say nothing a person watching would act on, and a live
/// panel that narrates its own bookkeeping is noise, not progress.
#[derive(Debug, Clone, PartialEq)]
pub enum Update {
    /// Codex said something — narration mid-turn, or the answer at the end.
    Text(String),
    /// Codex ran a shell command. Worth surfacing on its own: it is the part of a
    /// Codex turn that touches the machine, and it happens inside *its* sandbox
    /// rather than under this app's approval flow.
    Command(String),
}

/// The reason a turn failed, when Codex puts it on **stdout** as a JSON event.
///
/// `explain` reads stderr, which is where startup failures land — a model the plan
/// does not carry, a stale login. A turn that starts and then fails reports itself
/// on the JSONL stream instead, as `error` and again inside `turn.failed`, and
/// nothing was reading for it: stderr came back empty, `explain` had nothing to
/// quote, and a usage limit with a stated reset time surfaced in the app as "Codex
/// failed without saying why".
pub(super) fn parse_error(line: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(line.trim()).ok()?;
    let msg = match v.get("type")?.as_str()? {
        "error" => v.get("message")?.as_str()?,
        "turn.failed" => v.get("error")?.get("message")?.as_str()?,
        _ => return None,
    };
    let msg = msg.trim();
    (!msg.is_empty()).then(|| msg.to_string())
}

/// Turn one JSONL line from `codex exec --json` into an update, if it carries one.
///
/// Unrecognized and malformed lines are `None` rather than an error: this is an
/// unversioned CLI stream, and a new event type appearing in some future release
/// should cost the user nothing. The answer is reconstructed from `Text` updates,
/// so silence here degrades to less narration, never to a failed turn.
pub(super) fn parse_event(line: &str) -> Option<Update> {
    let v: serde_json::Value = serde_json::from_str(line.trim()).ok()?;
    if v.get("type")?.as_str()? != "item.completed" {
        return None;
    }
    let item = v.get("item")?;
    match item.get("type")?.as_str()? {
        "agent_message" => {
            let t = item.get("text")?.as_str()?.trim();
            (!t.is_empty()).then(|| Update::Text(t.to_string()))
        }
        "command_execution" => {
            let c = item.get("command")?.as_str()?.trim();
            (!c.is_empty()).then(|| Update::Command(c.to_string()))
        }
        _ => None,
    }
}

/// Run one turn through Codex and return what it answered.
///
/// The prompt arrives on stdin rather than as an argument: a conversation carries
/// newlines, quotes and occasionally something that looks like a flag, and argv is
/// the wrong place for all three. `-` tells the CLI to read it from stdin.
pub async fn run<F: FnMut(Update)>(model: &str, prompt: &str, cwd: &Path, mut on: F) -> Result<String, String> {
    let mut cmd = Command::new("codex");
    cmd.args(argv(model, cwd))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| match e.kind() {
        // The common one, and worth naming precisely: the provider is offered
        // because `auth.json` is on disk, which stays true after the CLI is
        // uninstalled. "No such file" alone would send someone to the wrong place.
        std::io::ErrorKind::NotFound => "The `codex` CLI isn't on PATH. Install it with `npm i -g @openai/codex`.".to_string(),
        _ => format!("Couldn't start Codex: {e}"),
    })?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(prompt.as_bytes()).await.map_err(|e| format!("Couldn't send the prompt to Codex: {e}"))?;
        // Codex waits for EOF before it starts, so this drop is load-bearing.
        drop(stdin);
    }

    // Drained on its own task rather than after stdout. A pipe whose reader is
    // busy elsewhere fills and then blocks the writer, so a chatty stderr would
    // stall Codex mid-turn while we sat reading stdout — a hang, not an error.
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

    // Read as they arrive: this loop is the whole point of `--json`. Each line is
    // handed on the moment it lands, so the panel fills while Codex is still
    // working instead of after it stops.
    let mut answer = String::new();
    // The last failure Codex reported on the JSONL stream, if any — see `parse_error`.
    let mut failure: Option<String> = None;
    if let Some(stdout) = child.stdout.take() {
        let mut lines = BufReader::new(stdout).lines();
        while let Some(line) = lines.next_line().await.map_err(|e| format!("Lost Codex's output: {e}"))? {
            if let Some(why) = parse_error(&line) {
                failure = Some(why);
                continue;
            }
            match parse_event(&line) {
                Some(Update::Text(t)) => {
                    // The last thing Codex says is the answer; everything before it
                    // was narration on the way there. Kept rather than concatenated,
                    // so the reply isn't the running commentary glued together.
                    answer = t.clone();
                    on(Update::Text(t));
                }
                Some(u) => on(u),
                None => {}
            }
        }
    }

    let status = child.wait().await.map_err(|e| format!("Codex didn't finish: {e}"))?;
    let stderr = match stderr_task {
        Some(t) => t.await.unwrap_or_default(),
        None => String::new(),
    };

    if !status.success() || answer.trim().is_empty() {
        // What Codex said about its own turn beats anything inferred from stderr:
        // it is the specific reason, and it is often directly actionable ("try
        // again at 2:44 PM").
        return Err(failure.unwrap_or_else(|| explain(&stderr, model)));
    }
    Ok(answer)
}
