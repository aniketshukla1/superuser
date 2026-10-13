use super::super::*;

pub(in crate::core::api_server) fn registry(ctx: &ApiContext) -> crate::core::personas::PersonaRegistry {
    use tauri::Manager;
    let dir = ctx.app.state::<crate::AppState>().data_dir.clone();
    crate::core::personas::PersonaRegistry::load(&dir)
}
