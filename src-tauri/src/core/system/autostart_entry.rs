use super::*;

pub(super) const LAUNCH_LABEL: &str = "ai.superuser.app";

/// Where the OS looks for "start this at login".
pub fn autostart_path() -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    if cfg!(target_os = "macos") {
        return Some(home.join("Library").join("LaunchAgents").join(format!("{LAUNCH_LABEL}.plist")));
    }
    if cfg!(target_os = "linux") {
        return Some(home.join(".config").join("autostart").join(format!("{LAUNCH_LABEL}.desktop")));
    }
    None
}

/// The launch-at-login entry for this platform.
///
/// Written from the CURRENT executable's path rather than a build-time constant:
/// a user who moves the app to a different folder would otherwise keep an entry
/// pointing at where it used to be, and see it silently stop launching.
pub fn autostart_entry(exe: &str) -> String {
    if cfg!(target_os = "linux") {
        return format!(
            "[Desktop Entry]\nType=Application\nName=Superuser\nExec={exe}\nX-GNOME-Autostart-enabled=true\n"
        );
    }
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key><string>{LAUNCH_LABEL}</string>
  <key>ProgramArguments</key><array><string>{exe}</string></array>
  <key>RunAtLoad</key><true/>
</dict>
</plist>
"#
    )
}

pub fn autostart_enabled() -> bool {
    autostart_path().map(|p| p.exists()).unwrap_or(false)
}

pub fn set_autostart(on: bool) -> Result<bool, String> {
    let Some(path) = autostart_path() else {
        return Err("Launch at login isn't supported on this platform yet.".into());
    };
    if !on {
        let _ = std::fs::remove_file(&path);
        return Ok(false);
    }
    let exe = std::env::current_exe()
        .map_err(|e| format!("couldn't find the app's own path: {e}"))?
        .to_string_lossy()
        .to_string();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("couldn't create {}: {e}", dir.display()))?;
    }
    std::fs::write(&path, autostart_entry(&exe)).map_err(|e| format!("couldn't write {}: {e}", path.display()))?;
    Ok(true)
}

/// The process holding sleep off, if any.
pub(super) fn awake_guard() -> &'static Mutex<Option<std::process::Child>> {
    static G: std::sync::OnceLock<Mutex<Option<std::process::Child>>> = std::sync::OnceLock::new();
    G.get_or_init(Default::default)
}

/// The command that stops the machine sleeping, per platform.
pub fn keep_awake_command() -> Option<(&'static str, Vec<&'static str>)> {
    if cfg!(target_os = "macos") {
        // -d display, -i idle: the app is working, so the screen should not dim
        // mid-run either.
        return Some(("caffeinate", vec!["-di"]));
    }
    if cfg!(target_os = "linux") {
        return Some(("systemd-inhibit", vec!["--what=idle:sleep", "--why=Superuser is working", "sleep", "infinity"]));
    }
    None
}

pub fn keep_awake_enabled() -> bool {
    let mut g = awake_guard().lock().unwrap();
    match g.as_mut() {
        None => false,
        Some(c) => match c.try_wait() {
            // It exited on its own; the machine is no longer being held awake, and
            // reporting otherwise would be a toggle that lies.
            Ok(Some(_)) => {
                *g = None;
                false
            }
            Ok(None) => true,
            Err(_) => false,
        },
    }
}

pub fn set_keep_awake(on: bool) -> Result<bool, String> {
    let mut g = awake_guard().lock().unwrap();
    if let Some(mut c) = g.take() {
        let _ = c.kill();
        let _ = c.wait();
    }
    if !on {
        return Ok(false);
    }
    let Some((program, args)) = keep_awake_command() else {
        return Err("Keeping the machine awake isn't supported on this platform yet.".into());
    };
    let child = std::process::Command::new(program)
        .args(&args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("couldn't start `{program}`: {e}"))?;
    *g = Some(child);
    Ok(true)
}

/// What the update banner is told.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UpdateStatus {
    /// An update is available and ready to be fetched.
    pub available: bool,
    pub version: Option<String>,
    pub notes: Option<String>,
    /// Why there is nothing to report, when there isn't. Distinguishes "you are up
    /// to date" from "this build has no update channel", which a null could not.
    pub reason: Option<String>,
}

impl UpdateStatus {
    pub fn none(reason: &str) -> UpdateStatus {
        UpdateStatus { available: false, version: None, notes: None, reason: Some(reason.into()) }
    }
}
