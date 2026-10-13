//! OAuth connectors (BYO app registration, no broker)


/// Save the app registration the user created with a vendor (client id/secret).
#[tauri::command]
pub fn oauth_set_client(id: String, client_id: String, client_secret: Option<String>) -> Result<(), String> {
    crate::integrations::oauth2::find_spec(&id).ok_or_else(|| format!("{id} isn't an OAuth connector."))?;
    if client_id.trim().is_empty() {
        return Err("the client id is empty".into());
    }
    crate::integrations::oauth2::set_client(&id, client_id.trim(), client_secret.unwrap_or_default().trim());
    Ok(())
}

/// Whether a connector already has an app registration stored.
#[tauri::command]
pub fn oauth_has_client(id: String) -> bool {
    crate::integrations::oauth2::has_client(&id)
}

/// Run the OAuth handshake in the browser and store the tokens.
#[tauri::command]
pub async fn oauth_connect(app: tauri::AppHandle, id: String) -> Result<String, String> {
    let spec = crate::integrations::oauth2::find_spec(&id).ok_or_else(|| format!("{id} isn't an OAuth connector."))?;
    crate::integrations::oauth2::connect(app, spec).await
}
