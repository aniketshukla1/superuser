use super::*;

pub struct TermSession {
    pub writer: Box<dyn Write + Send>,
    pub master: Box<dyn MasterPty + Send>,
    pub child: Box<dyn Child + Send + Sync>,
}

/// An agent command in flight: everything the pty has printed since we wrote it,
/// and the exit code once the sentinel shows up.
pub(super) struct Capture {
    pub(super) nonce: String,
    pub(super) buf: String,
    pub(super) code: Option<i32>,
}

#[derive(Default)]
pub(super) struct Shared {
    pub(super) session: Mutex<Option<TermSession>>,
    /// One agent command at a time. A second one waits its turn rather than
    /// interleaving its output with the first.
    pub(super) capture: Mutex<Option<Capture>>,
    pub(super) done: Condvar,
}

pub(super) fn shared() -> &'static Shared {
    static S: OnceLock<Shared> = OnceLock::new();
    S.get_or_init(Shared::default)
}

/// Lock, recovering from poisoning. These globals outlive any single turn, and the
/// app runs for weeks: one panic somewhere must not leave the terminal permanently
/// unopenable. The guarded data is a buffer and a session handle — a torn write to
/// either is worth far less than losing the shell for the rest of the process.
pub(super) fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

pub(super) fn size(cols: u16, rows: u16) -> PtySize {
    PtySize { rows: rows.max(1), cols: cols.max(1), pixel_width: 0, pixel_height: 0 }
}

/// Spawn a login shell in a fresh PTY rooted at `cwd`. Returns the reader half; the
/// caller wires the reader thread (it needs an AppHandle to emit) and must feed every
/// chunk back through [`feed`] so agent commands can see their own output.
pub fn open(cwd: &str, cols: u16, rows: u16) -> Result<Box<dyn std::io::Read + Send>, String> {
    close(); // replace any existing session
    let pair = native_pty_system().openpty(size(cols, rows)).map_err(|e| e.to_string())?;
    // Pick the platform's shell. portable-pty uses ConPTY on Windows.
    #[cfg(windows)]
    let (shell, login) = ("powershell.exe".to_string(), false);
    #[cfg(not(windows))]
    let (shell, login) = (std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into()), true);
    let mut cmd = CommandBuilder::new(&shell);
    if login { cmd.arg("-l"); } // login shell → user's full PATH (unix)
    cmd.cwd(cwd);
    cmd.env("TERM", "xterm-256color");
    let child = pair.slave.spawn_command(cmd).map_err(|e| e.to_string())?;
    drop(pair.slave); // parent doesn't keep the slave end
    let reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;
    let writer = pair.master.take_writer().map_err(|e| e.to_string())?;
    *lock(&shared().session) = Some(TermSession { writer, master: pair.master, child });
    Ok(reader)
}

pub fn is_open() -> bool {
    lock(&shared().session).is_some()
}

pub fn write(data: &str) {
    if let Some(s) = lock(&shared().session).as_mut() {
        let _ = s.writer.write_all(data.as_bytes());
        let _ = s.writer.flush();
    }
}

pub fn resize(cols: u16, rows: u16) {
    if let Some(s) = lock(&shared().session).as_ref() {
        let _ = s.master.resize(size(cols, rows));
    }
}

pub fn close() {
    if let Some(mut s) = lock(&shared().session).take() {
        let _ = s.child.kill();
    }
    // Anything waiting on a sentinel will never see it now.
    *lock(&shared().capture) = None;
    shared().done.notify_all();
}

/// Every chunk the reader thread pulls off the pty passes through here on its way
/// to the UI. When an agent command is in flight this is how it learns what its own
/// command printed, and when it finished.
pub fn feed(chunk: &str) {
    let sh = shared();
    let mut guard = lock(&sh.capture);
    let Some(cap) = guard.as_mut() else { return };
    cap.buf.push_str(chunk);
    // The marker also appears in the shell's echo of the line we typed — but there it
    // still reads `%d`, because that copy was never handed to printf. Requiring digits
    // is what tells the real completion apart from its own echo.
    //
    // Every occurrence has to be checked, not just the first: the echo always arrives
    // ahead of the real thing, so stopping at match one means never finding the marker
    // and hanging until the timeout.
    let head = format!("__SU_END_{}_", cap.nonce);
    for (at, _) in cap.buf.match_indices(&head) {
        let rest = &cap.buf[at + head.len()..];
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if !digits.is_empty() && rest[digits.len()..].starts_with("__") {
            cap.code = Some(digits.parse().unwrap_or(-1));
            sh.done.notify_all();
            break;
        }
    }
}

/// Run `cmd` in the shared shell and wait for it to finish, returning
/// `(exit_code, output)`. `None` when no terminal is open — the caller falls back to
/// a detached process, so the agent still works when the panel was never opened.
pub fn run_captured(cmd: &str, timeout_secs: u64) -> Option<(i32, String)> {
    if !is_open() {
        return None;
    }
    let sh = shared();
    let nonce = format!("{:x}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).ok()?.as_nanos());

    {
        let mut guard = lock(&sh.capture);
        if guard.is_some() {
            return None; // another agent command owns the shell; don't interleave
        }
        *guard = Some(Capture { nonce: nonce.clone(), buf: String::new(), code: None });
    }

    // `printf` rather than `echo` so the format string survives the shell's echo of the
    // line intact, and `$?` is the status of the user's command, not of printf.
    write(&format!("{cmd}\nprintf '\\n__SU_END_{nonce}_%d__\\n' $?\n"));

    let deadline = Instant::now() + Duration::from_secs(timeout_secs);
    let mut guard = lock(&sh.capture);
    loop {
        match guard.as_ref() {
            Some(c) if c.code.is_some() => break,
            None => return None, // terminal closed under us
            _ => {}
        }
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            let out = guard.take().map(|c| c.buf).unwrap_or_default();
            return Some((124, format!("{}\n[timed out after {timeout_secs}s]", clean(&out, &nonce))));
        }
        let (g, _) = sh.done.wait_timeout(guard, left).unwrap_or_else(|e| e.into_inner());
        guard = g;
    }
    let cap = guard.take()?;
    Some((cap.code.unwrap_or(-1), clean(&cap.buf, &nonce)))
}

/// Turn raw pty bytes into something worth handing a model: drop the echoed command
/// line and the sentinel machinery, strip ANSI so escape codes don't eat the context
/// window. The UI keeps the raw stream — the colour is the point on screen.
pub(super) fn clean(raw: &str, nonce: &str) -> String {
    let stripped = strip_ansi(raw);
    let mut lines: Vec<&str> = stripped.lines().collect();
    // Drop every line mentioning the sentinel: the echoed `printf ...` and the marker itself.
    let marker = format!("__SU_END_{nonce}_");
    lines.retain(|l| !l.contains(&marker));
    // The shell echoes the command back before running it; that first line is the
    // prompt + what we typed, which the model already knows.
    if !lines.is_empty() {
        lines.remove(0);
    }
    lines.join("\n").trim_end().to_string()
}
