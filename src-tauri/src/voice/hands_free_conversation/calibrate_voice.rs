use super::super::*;

#[tauri::command]
pub fn stop_conversation(state: tauri::State<AppState>) {
    state.dictation.conversing.store(false, Ordering::SeqCst);
    state.speaker.cancel_all();
    state.speaker.stop();
    let _ = end_capture(&state);
}

#[tauri::command]
pub fn is_conversing(state: tauri::State<AppState>) -> bool {
    state.dictation.conversing.load(Ordering::SeqCst)
}

/// What each voice engine can do here, and why it cannot when it can't.
///
/// The UI needs this to tell the truth: an engine the user picks but hasn't
/// installed should say so in the picker, not fail silently at the first sentence.
#[tauri::command]
pub fn voice_engines(state: tauri::State<AppState>) -> Vec<crate::core::tts::EngineStatus> {
    use crate::core::tts::{engine_status, Engine};
    [Engine::System, Engine::Piper, Engine::Kokoro, Engine::Cloud]
        .iter()
        .map(|e| engine_status(&state.data_dir, *e))
        .collect()
}

/// Measure this room, and remember it.
///
/// Called from Settings ▸ Voice input, where the user is told what is happening and
/// can stay quiet for it. Thresholds are meaningless in the abstract: the level
/// meter reads voice, room and device gain together, and that mixture differs by an
/// order of magnitude between machines. One measurement, made deliberately, beats a
/// guess made silently at the start of every conversation.
#[tauri::command]
pub fn calibrate_voice(app: tauri::AppHandle) -> Result<crate::core::conversation::Calibration, String> {
    use crate::core::conversation::{floor_of, save_calibration, Calibration};
    let already_open = {
        let state = app.state::<AppState>();
        state.dictation.recording.load(Ordering::SeqCst)
    };
    if !already_open {
        let state = app.state::<AppState>();
        start_dictation(state)?;
    }
    // Let the device settle: the first frames after a microphone opens are not the
    // room, and a calibration built from them measures the driver, not the space.
    std::thread::sleep(std::time::Duration::from_millis(200));

    let mut samples: Vec<f32> = Vec::with_capacity(60);
    for _ in 0..60 {
        {
            let state = app.state::<AppState>();
            samples.push(state.dictation.level());
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }

    let state = app.state::<AppState>();
    if !already_open {
        // Leave the microphone exactly as we found it — a settings screen must not
        // leave a recording running behind it.
        let _ = end_capture(&state);
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);
    let cal = Calibration::from_floor(floor_of(samples), now);
    save_calibration(&state.data_dir, &cal).map_err(|e| format!("Couldn't save the measurement: {e}"))?;
    Ok(cal)
}

/// The saved measurement, if the user has made one.
#[tauri::command]
pub fn voice_calibration(state: tauri::State<AppState>) -> Option<crate::core::conversation::Calibration> {
    crate::core::conversation::load_calibration(&state.data_dir)
}
