use super::super::*;

/// Start listening continuously and reporting turns.
///
/// The microphone stays open. A watcher thread reads the level meter, decides where
/// turns begin and end, and transcribes each finished turn — so the user talks when
/// they want to rather than holding a button.
///
/// Three things this thread must not do: block on transcription (the next turn
/// starts while the last is still being written), grow the buffer forever (an open
/// mic is an unbounded recording), or trust the level meter while the assistant is
/// speaking (the speakers feed the mic, and its own voice would interrupt it).
#[tauri::command]
pub fn start_conversation(app: tauri::AppHandle) -> Result<DictationStatus, String> {
    {
        let state = app.state::<AppState>();
        if state.dictation.conversing.load(Ordering::SeqCst) {
            return Ok(status_of(&state));
        }
    }
    // Reuse the ordinary capture path — one microphone, one way in.
    {
        let state = app.state::<AppState>();
        start_dictation(state)?;
    }
    {
        let state = app.state::<AppState>();
        state.dictation.conversing.store(true, Ordering::SeqCst);
    }
    let _ = app.emit("voice-state", serde_json::json!({ "state": "listening" }));

    let handle = app.clone();
    std::thread::spawn(move || {
        use crate::core::conversation::{
            pre_roll_samples, turn_start_index, EchoGate, TurnConfig, TurnDetector, TurnEvent,
        };
        // Let the capture settle before measuring — the first frames after a device
        // opens are not the room.
        std::thread::sleep(std::time::Duration::from_millis(150));
        const FRAME_MS: u32 = 20;

        // Thresholds only mean anything relative to the room they are used in, so
        // use the measurement made in Settings. Measuring here instead would put
        // three quarters of a second of dead air before every session and assume
        // the user happened to be silent at that moment.
        //
        // Falling back to measuring keeps voice working for someone who never
        // visited Settings; it is the worse path, not the absent one.
        let saved = {
            let state = handle.state::<AppState>();
            crate::core::conversation::load_calibration(&state.data_dir)
        };
        let saved_was_present = saved.is_some();
        let base = match saved {
            Some(c) => c.to_config(),
            None => {
                let mut samples: Vec<f32> = Vec::with_capacity(30);
                for _ in 0..30 {
                    {
                        let state = handle.state::<AppState>();
                        if !state.dictation.conversing.load(Ordering::SeqCst) {
                            return;
                        }
                        samples.push(state.dictation.level());
                    }
                    std::thread::sleep(std::time::Duration::from_millis(20));
                }
                TurnConfig::for_noise_floor(crate::core::conversation::floor_of(samples))
            }
        };
        let _ = handle.emit(
            "voice-calibrated",
            serde_json::json!({
                "start_level": base.start_level,
                "end_level": base.end_level,
                "from_settings": saved_was_present,
            }),
        );
        let mut detector = TurnDetector::new(base);
        let mut speaking_mode = false;
        // Where in the rolling buffer this turn began.
        let mut turn_start: usize = 0;
        // How much of the buffer has already been consumed by finished turns.
        let mut consumed: usize = 0;
        // Without echo cancellation the mic records the assistant too. This tracks
        // when its voice is still in the room, and `barrier` marks how far into the
        // buffer that recording reaches — no user turn may begin before it.
        let mut echo = EchoGate::default();
        let mut barrier: usize = 0;

        loop {
            let state = handle.state::<AppState>();
            if !state.dictation.conversing.load(Ordering::SeqCst) {
                break;
            }
            let level = state.dictation.level();
            let buffered = state.dictation.samples.lock().map(|b| b.len()).unwrap_or(0);
            let rate = state.dictation.source_rate.load(Ordering::SeqCst).max(1);

            // "Is its voice still in the room" rather than "is a player process
            // alive": the gap between two sentences of one answer is not silence,
            // and neither is the moment after the last one.
            let assistant_audible = echo.tick(state.speaker.speaking(), FRAME_MS);
            if assistant_audible {
                // Everything recorded up to now may contain the assistant. Pushing
                // the barrier forward each frame is what keeps its own words out of
                // the user's next turn — the part raising thresholds cannot do.
                barrier = buffered;
            }
            // Swap thresholds when its voice arrives or leaves, so it cannot trip
            // the detector but a person talking over it still can.
            if assistant_audible != speaking_mode {
                speaking_mode = assistant_audible;
                let cfg = if speaking_mode { base.while_speaking() } else { base };
                detector = TurnDetector::new(cfg);
            }

            match detector.push(level, FRAME_MS) {
                TurnEvent::SpeechStarted => {
                    // Rewind for the first syllable, but never back past the
                    // assistant's own recording.
                    turn_start = turn_start_index(buffered, pre_roll_samples(rate), consumed, barrier);
                    if assistant_audible {
                        // Barge-in: stop the sentence in flight and everything queued.
                        state.speaker.cancel_all();
                        state.speaker.stop();
                        let _ = handle.emit("voice-bargein", serde_json::json!({}));
                    }
                    let _ = handle.emit("voice-state", serde_json::json!({ "state": "hearing" }));
                }
                TurnEvent::SpeechEnded => {
                    // Re-apply the barrier at the end too: the assistant may have
                    // started speaking again part-way through this turn.
                    let from = turn_start.max(barrier.min(buffered));
                    let audio: Vec<f32> = state
                        .dictation
                        .samples
                        .lock()
                        .ok()
                        .map(|b| b.get(from..buffered).map(|s| s.to_vec()).unwrap_or_default())
                        .unwrap_or_default();
                    consumed = buffered;
                    let _ = handle.emit("voice-state", serde_json::json!({ "state": "thinking" }));
                    // Transcribe off this thread: the user may already be starting
                    // their next sentence, and the mic must keep listening.
                    let h2 = handle.clone();
                    std::thread::spawn(move || {
                        let st = h2.state::<AppState>();
                        let channels = 1u16; // the capture buffer is already interleaved per device
                        let mono = to_whisper_mono(&audio, channels, rate);
                        if mono.len() < (dictation::TARGET_RATE / 4) as usize {
                            let _ = h2.emit("voice-state", serde_json::json!({ "state": "listening" }));
                            return;
                        }
                        match transcribe(&st.data_dir, &mono) {
                            Ok(text) if !text.trim().is_empty() => {
                                let _ = h2.emit("voice-turn", serde_json::json!({ "text": text }));
                            }
                            // Silence, or a turn that held no words: say nothing
                            // rather than sending an empty message to the model.
                            Ok(_) => {
                                let _ = h2.emit("voice-state", serde_json::json!({ "state": "listening" }));
                            }
                            Err(e) => {
                                let _ = h2.emit("voice-error", serde_json::json!({ "message": e }));
                                let _ = h2.emit("voice-state", serde_json::json!({ "state": "listening" }));
                            }
                        }
                    });
                }
                TurnEvent::Nothing => {}
            }

            // An open microphone is an unbounded recording. Once a turn is finished
            // and nothing is in flight, drop what has been consumed so a long
            // conversation does not grow until the machine complains.
            if !detector.speaking() && consumed > rate as usize * 30 {
                if let Ok(mut buf) = state.dictation.samples.lock() {
                    let cut = consumed.min(buf.len());
                    buf.drain(..cut);
                    // The barrier is an index into that buffer, so trimming has to
                    // move it too — otherwise it points at the wrong audio, or past
                    // the end, and stops protecting anything.
                    barrier = barrier.saturating_sub(cut);
                }
                turn_start = 0;
                consumed = 0;
            }

            drop(state);
            std::thread::sleep(std::time::Duration::from_millis(FRAME_MS as u64));
        }
        let _ = handle.emit("voice-state", serde_json::json!({ "state": "off" }));
    });

    let state = app.state::<AppState>();
    Ok(status_of(&state))
}
