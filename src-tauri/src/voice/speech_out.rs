//! Speech out

use super::*;

/// The system voices installed on this machine.
///
/// Parsed from the OS's own list rather than hard-coded: the set differs per
/// machine and per language, and a fixed list would offer voices that aren't there.
#[tauri::command]
pub fn list_voices() -> Vec<String> {
    #[cfg(target_os = "macos")]
    {
        let out = std::process::Command::new("say").arg("-v").arg("?").output();
        if let Ok(o) = out {
            return String::from_utf8_lossy(&o.stdout)
                .lines()
                .filter_map(|l| {
                    // "Daniel              en_GB    # Hello there."
                    let name = l.split("  ").next()?.trim();
                    // English voices only: a reply spoken by a voice for another
                    // language is unintelligible rather than merely accented.
                    if name.is_empty() || !l.contains("en_") {
                        return None;
                    }
                    Some(name.to_string())
                })
                .collect();
        }
    }
    Vec::new()
}

/// Speak an answer, sentence by sentence, as a stoppable background job.
///
/// The chunking is the point: the first sentence is spoken while the caller may
/// still be appending to `text`, and each chunk is a separate child process, so
/// barge-in kills the sentence in flight and `cancel_all` stops the ones behind it.
#[tauri::command]
pub fn speak(
    app: tauri::AppHandle,
    text: String,
    engine: Option<String>,
    voice: Option<String>,
    persona: Option<String>,
) -> Result<(), String> {
    let state = app.state::<AppState>();
    let engine = crate::core::tts::Engine::parse(engine.as_deref().unwrap_or("system"));
    if engine.leaves_machine() && state.settings.lock().unwrap().local_only {
        return Err("This voice sends text off the machine, and local-only mode is on.".into());
    }
    // An explicit voice wins; otherwise the persona picks one, so you can hear who
    // is answering without looking.
    let voice = voice.filter(|v| !v.trim().is_empty()).or_else(|| {
        persona
            .as_deref()
            .and_then(|p| crate::core::conversation::voice_for_persona(p, &list_voices()))
    });
    let spoken = crate::core::tts::speakable(&text);
    if spoken.trim().is_empty() {
        return Ok(());
    }
    // Split into sentences here and queue them, so a caller can hand over an answer
    // one sentence at a time as it streams and hear it in order.
    {
        let state = app.state::<AppState>();
        let mut rest = spoken.as_str();
        while let Some((chunk, used)) = crate::core::tts::next_chunk(rest, true) {
            state.speaker.enqueue(&chunk);
            rest = &rest[used..];
            if rest.trim().is_empty() {
                break;
            }
        }
        // Only the first caller starts a worker; the rest just added to its queue.
        if !state.speaker.claim_drain() {
            return Ok(());
        }
        state.speaker.begin();
    }

    let handle = app.clone();
    std::thread::spawn(move || {
        let mut unsaid_spoken = String::new();
        loop {
            let state = handle.state::<AppState>();
            if state.speaker.cancelled() {
                break;
            }
            let Some(chunk) = state.speaker.next_queued() else {
                // The queue is empty, but a streaming answer may still be writing
                // its next sentence. Wait a beat before deciding the answer is over,
                // otherwise every pause between sentences would end the speech.
                drop(state);
                std::thread::sleep(std::time::Duration::from_millis(120));
                let state = handle.state::<AppState>();
                if state.speaker.queue_len() == 0 {
                    break;
                }
                continue;
            };
            let data_dir = state.data_dir.clone();
            if let Err(e) = state.speaker.speak(&data_dir, engine, voice.as_deref().unwrap_or(""), &chunk) {
                // Say why rather than going quiet: a missing engine is a setup
                // problem the user can fix, and silence looks like a broken app.
                let _ = handle.emit("voice-error", serde_json::json!({ "message": e }));
                break;
            }
            if !unsaid_spoken.is_empty() {
                unsaid_spoken.push(' ');
            }
            unsaid_spoken.push_str(&chunk);
            // Poll often enough that an interruption lands within a syllable.
            while state.speaker.speaking() {
                if state.speaker.cancelled() {
                    state.speaker.stop();
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(30));
            }
        }
        let state = handle.state::<AppState>();
        let interrupted = state.speaker.cancelled();
        let unheard: Vec<String> = std::iter::from_fn(|| state.speaker.next_queued()).collect();
        state.speaker.release_drain();
        let _ = handle.emit(
            "voice-spoke",
            serde_json::json!({
                "interrupted": interrupted,
                "heard": unsaid_spoken,
                "unheard": unheard.join(" "),
            }),
        );
        let _ = handle.emit("voice-state", serde_json::json!({ "state": "listening" }));
    });
    Ok(())
}

/// Barge-in: stop the sentence in flight AND the ones queued behind it.
#[tauri::command]
pub fn stop_speaking(state: tauri::State<AppState>) {
    state.speaker.cancel_all();
    state.speaker.stop();
}

#[tauri::command]
pub fn is_speaking(state: tauri::State<AppState>) -> bool {
    state.speaker.speaking()
}
