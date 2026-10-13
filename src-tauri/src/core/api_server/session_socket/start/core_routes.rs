use super::*;

/// The first half of the API's routes — health, sessions, skills, personas, memory, journal,
/// knowledge, workspaces, connectors and the profile. `start` merges it with the rest.
pub(super) fn core_routes() -> Router<Arc<ApiContext>> {
    Router::new()
        .route("/v1/health", get(health))
        .route("/v1/sessions/:id/skills", get(session_skills_get).post(session_skills_set))
        .route("/v1/subagents", get(subagents_list))
        .route("/v1/board", get(board_get))
        .route("/v1/feedback", get(feedback_get).post(feedback_post))
        .route("/v1/feedback/report", get(feedback_report_get))
        .route("/v1/unrouted", get(unrouted_list))
        .route("/v1/unrouted/clear", post(unrouted_clear))
        .route("/v1/risk-overrides", get(risk_overrides_get).post(risk_overrides_set))
        .route("/v1/workspaces/trusted", get(workspaces_trusted))
        .route("/v1/workspaces/trust", post(workspace_trust_set))
        .route("/v1/models", get(models))
        .route("/v1/chat/completions", post(chat_completions))
        // the GUI's session surfaces
        .route("/v1/stats", get(stats))
        .route("/v1/personas", get(personas_list).post(persona_create))
        .route("/v1/personas/install", post(persona_install))
        .route("/v1/import/package/preview", post(package_import_preview))
        .route("/v1/import/package", post(package_import_apply))
        .route("/v1/secrets/set-aside", post(secrets_set_aside))
        .route("/v1/skills", get(skills_list))
        .route("/v1/skills/preview", post(skill_preview))
        .route("/v1/skills/install", post(skill_install))
        .route("/v1/skills/:name", axum::routing::delete(skill_uninstall).patch(skill_update))
        .route("/v1/memory", get(memory_list))
        .route("/v1/memory/:filename", get(memory_get).delete(memory_delete))
        .route("/v1/journal", get(journal_get))
        .route("/v1/journal/dream", post(journal_dream))
        .route("/v1/journal/core", axum::routing::put(journal_set_core))
        .route("/v1/journal/undo", post(journal_undo))
        .route("/v1/knowledge", get(knowledge_list))
        .route("/v1/knowledge/:slug", axum::routing::delete(knowledge_delete))
        // POST as well as PATCH, because the client has always sent POST. Enabling
        // an assistant, un-surfacing one and "Set default" all went to a route
        // registered for PATCH only and came back 405, silently — the checkbox
        // flipped optimistically and the choice never landed.
        .route(
        "/v1/personas/:id",
        get(persona_detail).post(persona_update).patch(persona_update).delete(persona_delete),
    )
        // The detail page's own Enabled toggle posts here, and this existed nowhere.
        .route("/v1/personas/:id/enable", post(persona_enable))
        .route("/v1/personas/:id/remote/test", post(persona_remote_test))
        .route("/v1/personas/:id/connections", get(persona_connections).post(persona_connection_set))
        .route("/v1/workspaces/recent", get(workspaces_recent))
        .route("/v1/workspaces/open", post(workspace_open))
        .route("/v1/workspaces/pick", post(workspace_pick))
        .route("/v1/sessions", get(sessions_list))
        .route("/v1/sessions/:id", axum::routing::patch(session_patch).delete(session_delete))
        .route("/v1/sessions/:id/messages", get(session_messages))
        .route("/v1/sessions/:id/artifacts", get(session_artifacts))
        .route("/v1/sessions/:id/artifacts/read", get(session_artifact_read))
        .route("/v1/sessions/:id/artifacts/write", post(session_artifact_write))
        .route("/v1/sessions/:id/artifacts/upload", post(session_artifact_upload))
        .route("/v1/sessions/:id/artifacts/sheet", post(session_sheet_write))
        .route("/v1/sessions/:id/artifacts/doc", post(session_doc_write))
        .route("/v1/sessions/:id/run", get(session_run))
        .route("/v1/sessions/:id/connections", get(session_connections).post(session_connection_set))
        .route("/v1/connectors", get(connectors_list))
        .route("/v1/connectors/:name/connect", post(connector_connect_api))
        .route("/v1/connectors/:name/disconnect", post(connector_disconnect_api))
        .route("/v1/connectors/:name/sync", post(connector_sync_api))
        .route("/v1/connectors/:name/tools", get(connector_tools_api))
        // The other end of `core::unrouted`: who may reach the agent through a
        // connector, and what to do about someone who tried and was refused.
        .route("/v1/connectors/:name/allow", post(connector_allow))
        .route("/v1/connectors/:name/disallow", post(connector_disallow))
        .route("/v1/connectors/:name/unauthorized/:id", post(connector_unauthorized_resolve))
        .route("/v1/profile", get(profile_get))
        .route("/v1/profile/form", post(profile_save_form))
        .route("/v1/profile/learning", post(profile_set_learning))
        .route("/v1/profile/pin", post(profile_pin))
        .route("/v1/profile/forget", post(profile_forget))
}
