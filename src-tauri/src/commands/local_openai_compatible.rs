//! Local OpenAI-compatible API

use super::*;

#[derive(serde::Serialize)]
pub struct ApiStatus {
    pub running: bool,
    pub port: Option<u16>,
    pub token: Option<String>,
    pub base_url: Option<String>,
}

#[tauri::command]
pub fn api_status(state: State<AppState>) -> ApiStatus {
    match &*state.api.lock().unwrap() {
        Some(api) => ApiStatus {
            running: true,
            port: Some(api.port),
            token: Some(api.token.clone()),
            base_url: Some(format!("http://127.0.0.1:{}/v1", api.port)),
        },
        None => ApiStatus { running: false, port: None, token: None, base_url: None },
    }
}

/// Start the local API (loopback-only, token-guarded). Off by default — starting
/// it is an explicit user action.
#[tauri::command]
pub async fn api_start(app: tauri::AppHandle, state: State<'_, AppState>, port: Option<u16>) -> Result<ApiStatus, String> {
    if state.api.lock().unwrap().is_some() {
        return Err("The API is already running.".into());
    }
    let api = crate::core::api_server::start(app, port.unwrap_or(8765)).await?;
    let status = ApiStatus {
        running: true,
        port: Some(api.port),
        token: Some(api.token.clone()),
        base_url: Some(format!("http://127.0.0.1:{}/v1", api.port)),
    };
    crate::core::audit::log(&state.data_dir, "connect", "user", "local-api", &format!("started on 127.0.0.1:{}", api.port), true);
    *state.api.lock().unwrap() = Some(api);
    Ok(status)
}

#[tauri::command]
pub fn api_stop(state: State<AppState>) -> bool {
    let mut guard = state.api.lock().unwrap();
    match guard.take() {
        Some(mut api) => {
            api.stop();
            crate::core::audit::log(&state.data_dir, "connect", "user", "local-api", "stopped", true);
            true
        }
        None => false,
    }
}
