use super::*;

/// Download the model, reporting progress as it goes.
#[tauri::command]
pub async fn download_dictation_model(app: tauri::AppHandle) -> Result<DictationStatus, String> {
    use futures_util::StreamExt;
    let state = app.state::<AppState>();
    if state.dictation.downloading.load(Ordering::SeqCst) {
        return Ok(status_of(&state));
    }
    let data_dir = state.data_dir.clone();
    state.dictation.downloading.store(true, Ordering::SeqCst);
    state.dictation.cancel_download.store(false, Ordering::SeqCst);
    let downloading = state.dictation.downloading.clone();
    let cancel = state.dictation.cancel_download.clone();

    let finish = |downloading: &Arc<std::sync::atomic::AtomicBool>| downloading.store(false, Ordering::SeqCst);

    let dir = data_dir.join("speech");
    std::fs::create_dir_all(&dir).map_err(|e| {
        finish(&downloading);
        format!("Couldn't create the model folder: {e}")
    })?;
    let target = dictation::model_path(&data_dir);
    let part = target.with_extension("part");

    let client = crate::core::egress::client("speech-model");
    let resp = match client.get(dictation::MODEL_URL).send().await {
        Ok(r) => r,
        Err(e) => {
            finish(&downloading);
            return Err(format!("Couldn't reach the model host: {e}"));
        }
    };
    let total = resp.content_length().unwrap_or(dictation::MODEL_BYTES);
    let mut file = std::fs::File::create(&part).map_err(|e| {
        finish(&downloading);
        format!("Couldn't write the model file: {e}")
    })?;
    let mut got: u64 = 0;
    let mut stream = resp.bytes_stream();
    use std::io::Write;
    while let Some(chunk) = stream.next().await {
        if cancel.load(Ordering::SeqCst) {
            drop(file);
            let _ = std::fs::remove_file(&part);
            finish(&downloading);
            return Ok(status_of(&state));
        }
        let chunk = match chunk {
            Ok(c) => c,
            Err(e) => {
                drop(file);
                let _ = std::fs::remove_file(&part);
                finish(&downloading);
                return Err(format!("The download broke: {e}"));
            }
        };
        if let Err(e) = file.write_all(&chunk) {
            drop(file);
            let _ = std::fs::remove_file(&part);
            finish(&downloading);
            return Err(format!("Couldn't write the model: {e}"));
        }
        got += chunk.len() as u64;
        let _ = app.emit("dictation-download-progress", DownloadProgress { downloaded_bytes: got, total_bytes: total });
    }
    drop(file);
    // Only now does it become the real file: a rename at the end means an
    // interrupted download can never be mistaken for an installed model.
    std::fs::rename(&part, &target).map_err(|e| {
        finish(&downloading);
        format!("Couldn't finish installing the model: {e}")
    })?;
    finish(&downloading);
    if dictation::model_looks_complete(&data_dir) {
        let _ = dictation::mark_verified(&data_dir);
    }
    Ok(status_of(&state))
}

#[tauri::command]
pub fn cancel_dictation_model_download(state: tauri::State<AppState>) {
    state.dictation.cancel_download.store(true, Ordering::SeqCst);
}

/// Check the installed model is whole, and remember the answer.
#[tauri::command]
pub fn verify_dictation_model(state: tauri::State<AppState>) -> Result<DictationStatus, String> {
    if !dictation::model_looks_complete(&state.data_dir) {
        return Err("The speech model looks incomplete — download it again.".into());
    }
    dictation::mark_verified(&state.data_dir).map_err(|e| format!("Couldn't record the check: {e}"))?;
    Ok(status_of(&state))
}

#[tauri::command]
pub fn mark_dictation_test_passed(state: tauri::State<AppState>) -> Result<DictationStatus, String> {
    dictation::mark_tested(&state.data_dir).map_err(|e| format!("Couldn't record the test: {e}"))?;
    Ok(status_of(&state))
}

#[tauri::command]
pub fn delete_dictation_model(state: tauri::State<AppState>) -> Result<DictationStatus, String> {
    dictation::forget_model(&state.data_dir);
    Ok(status_of(&state))
}
