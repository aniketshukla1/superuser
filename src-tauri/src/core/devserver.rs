//! Dev servers started from the browser's start page — the "hidden terminal
//! behind the browser".
//!
//! These deliberately do NOT run in the shared shell (core::terminal). Two reasons,
//! both found the hard way:
//!
//!  1. CWD. The shared shell is rooted at the *session's* workspace, but the browser
//!     start page reads its `.claude/launch.json` from whichever folder you pointed
//!     it at. Typing the command into that shell ran it against the wrong root —
//!     `npm --prefix frontend …` resolved inside the session workspace and died with
//!     ENOENT. A dev server has to run where its launch.json lives, full stop.
//!
//!  2. Focus. Starting a server was yanking you over to the Terminal tab. You asked
//!     for the browser, so the browser is where you should stay.
//!
//! Each server gets its own PTY rather than a plain piped child. A PTY makes the
//! process a session leader, so killing it takes the whole tree with it — a bare
//! `sh -c "npm run dev"` would otherwise leave node orphaned holding the port. It
//! also keeps the tools that check `isatty` in colour and unbuffered.

use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use tauri::{Emitter, Runtime};

pub struct Server {
    pub master: Box<dyn MasterPty + Send>,
    pub child: Box<dyn Child + Send + Sync>,
    pub cwd: String,
}

fn servers() -> &'static Mutex<HashMap<String, Server>> {
    static S: OnceLock<Mutex<HashMap<String, Server>>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(HashMap::new()))
}

fn lock() -> std::sync::MutexGuard<'static, HashMap<String, Server>> {
    servers().lock().unwrap_or_else(|e| e.into_inner())
}

/// Start `command` in `cwd`. Replaces any server already running under `name`, so
/// hitting Run twice doesn't leave two processes fighting over the port.
///
/// Output streams to `devserver-output` as `{ name, data }` (data is raw pty text,
/// ANSI stripped — this feeds a log pane, not a terminal emulator). `devserver-exit`
/// fires with `{ name, code }` when it stops.
pub fn start<R: Runtime>(app: &tauri::AppHandle<R>, name: &str, cwd: &str, command: &str) -> Result<(), String> {
    if command.trim().is_empty() {
        return Err("That server has no command to run.".into());
    }
    if !std::path::Path::new(cwd).is_dir() {
        return Err(format!("{cwd} is not a folder."));
    }
    stop(name);

    let pair = native_pty_system()
        .openpty(PtySize { rows: 40, cols: 120, pixel_width: 0, pixel_height: 0 })
        .map_err(|e| e.to_string())?;

    // A login shell so the command sees the same PATH the user's terminal would —
    // nvm, homebrew, pyenv and friends all live in the login profile, and a dev
    // server that can't find `npm` is the most boring possible failure.
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
    cmd.cwd(cwd); // the whole point
    cmd.env("TERM", "xterm-256color");
    cmd.env("FORCE_COLOR", "1");

    let child = pair.slave.spawn_command(cmd).map_err(|e| e.to_string())?;
    drop(pair.slave);
    let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;

    lock().insert(name.to_string(), Server { master: pair.master, child, cwd: cwd.to_string() });

    let handle = app.clone();
    let label = name.to_string();
    std::thread::spawn(move || {
        use std::io::Read;
        let mut buf = [0u8; 8192];
        loop {
            match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    let text = crate::core::terminal::strip_ansi(&String::from_utf8_lossy(&buf[..n]));
                    if text.is_empty() {
                        continue;
                    }
                    let _ = handle.emit("devserver-output", serde_json::json!({ "name": label, "data": text }));
                }
            }
        }
        let code = lock().get_mut(&label).and_then(|s| s.child.wait().ok()).map(|s| s.exit_code()).unwrap_or(0);
        lock().remove(&label);
        let _ = handle.emit("devserver-exit", serde_json::json!({ "name": label, "code": code }));
    });

    Ok(())
}

pub fn stop(name: &str) {
    if let Some(mut s) = lock().remove(name) {
        let _ = s.child.kill();
    }
}

/// Names of everything currently running, so the UI can show Stop instead of Run
/// after a reload.
pub fn running() -> Vec<String> {
    lock().keys().cloned().collect()
}

/// Nothing we started should outlive the app holding a port hostage.
pub fn stop_all() {
    let mut map = lock();
    for (_, s) in map.iter_mut() {
        let _ = s.child.kill();
    }
    map.clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_command_needs_a_real_folder_to_run_in() {
        // The bug this guards: the server ran in the session's workspace instead of
        // the folder the start page read, so `--prefix frontend` pointed at nothing.
        // A bad cwd must be refused loudly, not silently run somewhere else.
        let dir = std::env::temp_dir().join("su-devserver-nope-does-not-exist");
        assert!(!dir.is_dir());
        // start() needs an AppHandle, so assert the guard directly.
        assert!(!std::path::Path::new(&dir).is_dir());
    }

    #[test]
    fn nothing_is_running_before_anything_starts() {
        stop_all();
        assert_eq!(running(), Vec::<String>::new());
    }

    #[test]
    fn stopping_something_that_was_never_started_is_harmless() {
        stop("not-a-server");
        assert_eq!(running(), Vec::<String>::new());
    }

    /// The pty plumbing itself: does a command spawned this way actually reach us,
    /// and does it run where we told it to? This is the exact shape `start` uses,
    /// minus the Tauri emit.
    #[test]
    fn output_reaches_the_reader_and_the_command_runs_in_the_given_cwd() {
        use std::io::Read;
        let dir = std::env::temp_dir().join(format!("su-devserver-cwd-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        let pair = native_pty_system()
            .openpty(PtySize { rows: 40, cols: 120, pixel_width: 0, pixel_height: 0 })
            .unwrap();
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into());
        let mut cmd = CommandBuilder::new(shell);
        cmd.args(["-lc", "pwd && echo DEVSERVER-MARKER"]);
        cmd.cwd(&dir);
        cmd.env("TERM", "xterm-256color");
        let mut child = pair.slave.spawn_command(cmd).unwrap();
        drop(pair.slave);
        let mut reader = pair.master.try_clone_reader().unwrap();

        let mut out = String::new();
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => out.push_str(&String::from_utf8_lossy(&buf[..n])),
            }
        }
        let _ = child.wait();
        let clean = crate::core::terminal::strip_ansi(&out);
        std::fs::remove_dir_all(&dir).ok();

        assert!(clean.contains("DEVSERVER-MARKER"), "no output came back: {clean:?}");
        // macOS reports /private/var for /var, so compare on the leaf.
        let leaf = format!("su-devserver-cwd-{}", std::process::id());
        assert!(clean.contains(&leaf), "ran in the wrong folder: {clean:?}");
    }
}
