
/// Run a shell command in the workspace via a login shell (full PATH), capturing
/// combined stdout+stderr with a timeout and a size cap. Returns (exit_code,
/// output).
///
/// [`cmdgate::hard_deny`](crate::core::cmdgate::hard_deny) refuses the
/// catastrophes here as well as at the tool boundary. This is the lower of the
/// two checks and it stays deliberately: `run_capture` is a public function with
/// callers beyond the agent loop, and a floor that only exists one level up is
/// not a floor.
pub fn run_capture(root: &str, cmd: &str, timeout_secs: u64, cap: usize) -> (i32, String) {
    use std::io::Read;
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};

    if let Some(reason) = crate::core::cmdgate::hard_deny(cmd) {
        return (-1, format!("blocked: {reason}"));
    }
    let mut child = match Command::new("sh").arg("-lc").arg(cmd).current_dir(root)
        .stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()
    {
        Ok(c) => c,
        Err(e) => return (-1, format!("failed to start command: {e}")),
    };
    let mut out = child.stdout.take().unwrap();
    let mut err = child.stderr.take().unwrap();
    let oh = std::thread::spawn(move || { let mut s = String::new(); let _ = out.read_to_string(&mut s); s });
    let eh = std::thread::spawn(move || { let mut s = String::new(); let _ = err.read_to_string(&mut s); s });
    let deadline = Instant::now() + Duration::from_secs(timeout_secs);
    let mut timed_out = false;
    let code = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.code().unwrap_or(-1),
            Ok(None) => {
                if Instant::now() >= deadline { let _ = child.kill(); timed_out = true; break 124; }
                std::thread::sleep(Duration::from_millis(40));
            }
            Err(_) => break -1,
        }
    };
    let stdout = oh.join().unwrap_or_default();
    let stderr = eh.join().unwrap_or_default();
    let mut combined = stdout;
    if !stderr.is_empty() {
        if !combined.is_empty() { combined.push('\n'); }
        combined.push_str(&stderr);
    }
    // char-safe head+tail truncation so long logs don't blow the context window.
    if combined.chars().count() > cap {
        let chars: Vec<char> = combined.chars().collect();
        let half = cap / 2;
        let head: String = chars[..half].iter().collect();
        let tail: String = chars[chars.len() - half..].iter().collect();
        combined = format!("{head}\n… [output truncated] …\n{tail}");
    }
    if timed_out {
        combined = format!("[timed out after {timeout_secs}s]\n{combined}");
    }
    (code, combined)
}
