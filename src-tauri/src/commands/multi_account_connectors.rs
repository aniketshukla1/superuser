//! Multi-account connectors


/// Accounts registered for a connector (a pre-multi-account connection shows as
/// a single implicit "default").
#[tauri::command]
pub fn connector_accounts(connector: String) -> Vec<crate::integrations::accounts::Account> {
    crate::integrations::accounts::list(&connector)
}

/// Add (or relabel) an account, verifying the credential first when the
/// connector is one of the REST-registry ones.
#[tauri::command]
pub async fn connector_account_add(connector: String, account_id: String, label: String, credential: String) -> Result<String, String> {
    let identity = match crate::integrations::rest::find_spec(&connector) {
        Some(_) => crate::integrations::verify_rest(&connector, &credential).await?,
        None => label.clone(),
    };
    let label = if label.trim().is_empty() { identity.clone() } else { label };
    crate::integrations::accounts::add(&connector, &account_id, &label, &credential)?;
    Ok(identity)
}

#[tauri::command]
pub fn connector_account_remove(connector: String, account_id: String) -> bool {
    crate::integrations::accounts::remove(&connector, &account_id)
}

#[tauri::command]
pub fn connector_account_default(connector: String, account_id: String) -> Result<(), String> {
    crate::integrations::accounts::set_default(&connector, &account_id)
}
