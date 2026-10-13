use super::*;

/// Clear what a crashed browser leaves in its profile, so the next start is not refused.
///
/// Chrome writes `SingletonLock` as a link to `host-pid` and will not open a profile it
/// thinks is in use. After a crash that link outlives the process. The lock is removed
/// only when nothing is running under that pid — or when what is running is a Chrome
/// started on THIS profile, which can only be a browser this app left behind, and is
/// stopped. Anything else holding the pid is left alone, and Chrome's own refusal says so.
#[cfg(unix)]
pub fn clear_stale_locks(profile: &Path) {
    let lock = profile.join("SingletonLock");
    let Ok(target) = std::fs::read_link(&lock) else {
        return;
    };
    let pid: i32 = target.to_string_lossy().rsplit('-').next().and_then(|p| p.parse().ok()).unwrap_or(0);
    // SAFETY: signal 0 only asks whether the process exists.
    let alive = pid > 1 && unsafe { libc::kill(pid, 0) } == 0;
    if alive {
        let ours = std::process::Command::new("ps")
            .args(["-o", "command=", "-p", &pid.to_string()])
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).contains(&format!("--user-data-dir={}", profile.display())))
            .unwrap_or(false);
        if !ours {
            return;
        }
        unsafe { libc::kill(pid, libc::SIGKILL) };
        std::thread::sleep(std::time::Duration::from_millis(300));
    }
    for name in ["SingletonLock", "SingletonSocket", "SingletonCookie"] {
        let _ = std::fs::remove_file(profile.join(name));
    }
}

#[cfg(not(unix))]
pub fn clear_stale_locks(_profile: &Path) {}
