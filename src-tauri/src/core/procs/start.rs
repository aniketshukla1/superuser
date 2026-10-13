use super::*;

/// How much of a process's output is kept. The tail, because a dev server's useful
/// line is the most recent one; the count of what was dropped is kept alongside.
pub(super) const KEEP: usize = 64 * 1024;

/// How much one `process_read` may return. A read is a tool result, and a tool result
/// lands in the context window.
pub(super) const READ_CAP: usize = 8 * 1024;

pub const MAX_PER_SESSION: usize = 4;

#[derive(Default)]
pub(super) struct Output {
    pub(super) text: String,
    /// Characters dropped off the front to keep `text` under [`KEEP`].
    pub(super) dropped: usize,
    pub(super) exit: Option<i32>,
}

pub(super) struct Proc {
    pub(super) session: String,
    pub(super) command: String,
    pub(super) _master: Box<dyn MasterPty + Send>,
    pub(super) writer: Box<dyn Write + Send>,
    pub(super) child: Box<dyn Child + Send + Sync>,
    pub(super) out: Arc<Mutex<Output>>,
    /// Absolute offset already handed back to the model.
    pub(super) read_upto: usize,
}

pub(super) fn table() -> &'static Mutex<HashMap<String, Proc>> {
    static T: OnceLock<Mutex<HashMap<String, Proc>>> = OnceLock::new();
    T.get_or_init(|| Mutex::new(HashMap::new()))
}

pub(super) fn lock() -> std::sync::MutexGuard<'static, HashMap<String, Proc>> {
    table().lock().unwrap_or_else(|p| p.into_inner())
}

pub(super) fn new_handle() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(1);
    format!("p{}", N.fetch_add(1, Ordering::Relaxed))
}

/// Start `command` in `cwd` for `session`. Returns the handle.
pub fn start(session: &str, cwd: &str, command: &str) -> Result<String, String> {
    let command = command.trim();
    if command.is_empty() {
        return Err("error: give the command to start".into());
    }
    if let Some(reason) = crate::core::cmdgate::hard_deny(command) {
        return Err(format!("blocked: {reason}. Nothing was started."));
    }
    if !std::path::Path::new(cwd).is_dir() {
        return Err(format!("error: {cwd} is not a folder"));
    }
    let live = lock().values().filter(|p| p.session == session && p.out.lock().map(|o| o.exit.is_none()).unwrap_or(false)).count();
    if live >= MAX_PER_SESSION {
        return Err(format!(
            "error: {live} processes are already running in this conversation — stop one with process_stop first"
        ));
    }

    let pair = native_pty_system()
        .openpty(PtySize { rows: 40, cols: 120, pixel_width: 0, pixel_height: 0 })
        .map_err(|e| format!("error: could not open a terminal: {e}"))?;
    // A login shell, so the command finds the same PATH the user's terminal would —
    // nvm, homebrew and pyenv all live in the login profile.
    #[cfg(windows)]
    let mut cmd = {
        let mut c = CommandBuilder::new("powershell.exe");
        c.args(["-NoLogo", "-Command", command]);
        c
    };
    #[cfg(not(windows))]
    let mut cmd = {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into());
        let mut c = CommandBuilder::new(shell);
        c.args(["-lc", command]);
        c
    };
    cmd.cwd(cwd);
    cmd.env("TERM", "dumb");
    cmd.env("NO_COLOR", "1");

    let child = pair.slave.spawn_command(cmd).map_err(|e| format!("error: failed to start: {e}"))?;
    drop(pair.slave);
    let mut reader = pair.master.try_clone_reader().map_err(|e| format!("error: {e}"))?;
    let writer = pair.master.take_writer().map_err(|e| format!("error: {e}"))?;

    let out = Arc::new(Mutex::new(Output::default()));
    let sink = out.clone();
    std::thread::spawn(move || {
        use std::io::Read;
        let mut buf = [0u8; 8192];
        loop {
            match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    let text = crate::core::terminal::strip_ansi(&String::from_utf8_lossy(&buf[..n]));
                    if let Ok(mut o) = sink.lock() {
                        o.text.push_str(&text);
                        if o.text.len() > KEEP {
                            let cut = o.text.len() - KEEP;
                            let cut = (cut..o.text.len()).find(|i| o.text.is_char_boundary(*i)).unwrap_or(cut);
                            o.text.drain(..cut);
                            o.dropped += cut;
                        }
                    }
                }
            }
        }
    });

    let handle = new_handle();
    lock().insert(
        handle.clone(),
        Proc { session: session.into(), command: command.into(), _master: pair.master, writer, child, out, read_upto: 0 },
    );
    Ok(handle)
}

/// Poll the exit status without blocking, and record it.
pub(super) fn reap(p: &mut Proc) {
    if let Ok(Some(status)) = p.child.try_wait() {
        if let Ok(mut o) = p.out.lock() {
            o.exit.get_or_insert(status.exit_code() as i32);
        }
    }
}

/// Everything printed since the last read, and whether it is still running.
pub fn read(session: &str, handle: &str) -> Result<(String, Option<i32>), String> {
    let mut t = lock();
    let p = owned(&mut t, session, handle)?;
    reap(p);
    let o = p.out.lock().map_err(|_| "error: output unavailable".to_string())?;
    let end = o.dropped + o.text.len();
    let from = p.read_upto.max(o.dropped);
    let lost = from.saturating_sub(p.read_upto);
    let mut chunk = o.text[from - o.dropped..].to_string();
    let exit = o.exit;
    drop(o);
    // The most recent output is the useful output, so an over-long read keeps the end.
    if chunk.len() > READ_CAP {
        let cut = chunk.len() - READ_CAP;
        let cut = (cut..chunk.len()).find(|i| chunk.is_char_boundary(*i)).unwrap_or(cut);
        chunk = format!("[… {cut} earlier characters not shown …]\n{}", &chunk[cut..]);
    }
    if lost > 0 {
        chunk = format!("[… {lost} characters scrolled out before this read …]\n{chunk}");
    }
    p.read_upto = end;
    Ok((chunk, exit))
}

/// Write a line to the process's input.
pub fn send(session: &str, handle: &str, input: &str, newline: bool) -> Result<(), String> {
    // The same floor as the command itself. Without this, starting `bash` and sending
    // it the refused command would be a way round the gate.
    if let Some(reason) = crate::core::cmdgate::hard_deny(input) {
        return Err(format!("blocked: {reason}. Nothing was sent."));
    }
    let mut t = lock();
    let p = owned(&mut t, session, handle)?;
    reap(p);
    if p.out.lock().map(|o| o.exit.is_some()).unwrap_or(false) {
        return Err("error: that process has already exited — read its output, or start it again".into());
    }
    let line = if newline && !input.ends_with('\n') { format!("{input}\n") } else { input.to_string() };
    p.writer.write_all(line.as_bytes()).and_then(|_| p.writer.flush()).map_err(|e| format!("error: {e}"))
}

/// Stop it, and return whatever it printed that nobody has read yet.
pub fn stop(session: &str, handle: &str) -> Result<String, String> {
    let tail = read(session, handle).map(|(t, _)| t).unwrap_or_default();
    let mut t = lock();
    owned(&mut t, session, handle)?;
    if let Some(mut p) = t.remove(handle) {
        let _ = p.child.kill();
    }
    Ok(tail)
}
