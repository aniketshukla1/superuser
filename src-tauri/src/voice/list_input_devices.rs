use super::*;

/// cpal's stream is not `Send`-friendly to store directly, so the live capture is
/// owned by the thread that made it and told to stop through this handle.
pub(super) struct Capture {
    pub(super) stop: Arc<std::sync::atomic::AtomicBool>,
}

impl StreamHandle for Capture {
    fn stop(self: Box<Self>) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

/// One microphone, as the composer's device menu needs it.
#[derive(serde::Serialize, Clone, Debug, PartialEq)]
pub struct InputDevice {
    /// What cpal calls it. Also the id: names are what the OS gives us and what the
    /// user recognises, and there is no stable handle that survives a replug anyway.
    pub name: String,
    /// True for the one the OS would pick on its own.
    pub is_default: bool,
    /// False when the device enumerates but will not open — a virtual device whose
    /// host app is closed does this. Listing it and letting the recording fail later
    /// is worse than saying so in the menu.
    pub usable: bool,
}

/// Every input the machine currently has, default first.
///
/// Devices come and go — an iPhone mic appears when it is on the same desk and vanishes
/// when it leaves — so this is read fresh each time the menu opens rather than cached.
pub fn list_input_devices() -> Vec<InputDevice> {
    let host = cpal::default_host();
    let default_name = host
        .default_input_device()
        .and_then(|d| d.description().ok().map(|x| x.name().to_string()));
    let mut out: Vec<InputDevice> = Vec::new();
    if let Ok(devices) = host.input_devices() {
        for d in devices {
            let Ok(desc) = d.description() else { continue };
            let name = desc.name().to_string();
            // Duplicates are real: the same hardware can enumerate twice under
            // different hosts, and a menu with two identical rows is a menu you
            // cannot choose from.
            if out.iter().any(|x| x.name == name) {
                continue;
            }
            out.push(InputDevice {
                is_default: Some(&name) == default_name.as_ref(),
                usable: d.default_input_config().is_ok(),
                name,
            });
        }
    }
    out.sort_by_key(|d| (!d.is_default, d.name.to_ascii_lowercase()));
    out
}

/// The device to record from: the one that was chosen, or the system default.
///
/// Falls back rather than failing when the chosen one is gone. A microphone you picked
/// last week and unplugged since should not stop dictation from working today.
pub(super) fn preferred_device(preferred: &str) -> Option<cpal::Device> {
    let host = cpal::default_host();
    if !preferred.trim().is_empty() {
        if let Ok(devices) = host.input_devices() {
            for d in devices {
                if d.description().ok().map(|x| x.name().to_string()).as_deref() == Some(preferred.trim()) {
                    return Some(d);
                }
            }
        }
    }
    host.default_input_device()
}

/// The input device, and a sentence describing it for the settings screen.
pub(super) fn describe_device_named(preferred: &str) -> (Option<cpal::Device>, String, Option<String>) {
    match preferred_device(preferred) {
        None => (None, "No microphone found".into(), Some("This machine has no input device, so there is nothing to record from.".into())),
        Some(d) => {
            let name = d.description().map(|x| x.name().to_string()).unwrap_or_else(|_| "Microphone".into());
            match d.default_input_config() {
                Err(e) => (None, name.clone(), Some(format!("The microphone ({name}) could not be opened: {e}"))),
                Ok(cfg) => (Some(d), format!("{name} · {} Hz · {} ch", cfg.sample_rate(), cfg.channels()), None),
            }
        }
    }
}

pub(super) fn chosen_mic(state: &AppState) -> String {
    state.settings.lock().map(|s| s.mic_device.clone()).unwrap_or_default()
}

pub(super) fn status_of(state: &AppState) -> DictationStatus {
    let (_, summary, reason) = describe_device_named(&chosen_mic(state));
    state.dictation.status(&state.data_dir, summary, reason)
}

#[tauri::command]
pub fn list_mics() -> Vec<InputDevice> {
    list_input_devices()
}

#[tauri::command]
pub fn get_dictation_status(state: tauri::State<AppState>) -> DictationStatus {
    status_of(&state)
}

#[tauri::command]
pub fn dictation_level(state: tauri::State<AppState>) -> f32 {
    state.dictation.level()
}
