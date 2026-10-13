//! Remote MCP connectors (Claude-style)

use super::*;

#[tauri::command]
pub fn mcp_list(state: State<AppState>) -> Vec<crate::mcp::McpConnector> {
    state.mcp.list()
}

/// Begin OAuth for a remote MCP connector. Opens the browser, returns when the
/// loopback redirect is caught and tokens are stored.
#[tauri::command]
pub async fn mcp_connect(app: tauri::AppHandle, id: String, client_id: Option<String>) -> Result<(), String> {
    let connector = crate::mcp::connectors::find(&id).ok_or("Unknown connector")?;
    crate::mcp::oauth::connect(app, connector.id, connector.url, client_id).await
}

#[tauri::command]
pub fn mcp_disconnect(id: String) -> bool {
    crate::mcp::oauth::forget(&id)
}

/// Sync a connected MCP server into the Memory Tree.
#[tauri::command]
pub async fn mcp_sync(state: State<'_, AppState>, id: String) -> Result<u32, String> {
    let connector = crate::mcp::connectors::find(&id).ok_or("Unknown connector")?;
    let engine = crate::core::memory::MemoryEngine::new(state.memory.vault_path.clone());
    let out = crate::mcp::client::sync(&id, connector, &engine).await?;
    Ok(out.created)
}
