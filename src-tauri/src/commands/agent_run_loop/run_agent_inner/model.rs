use super::*;

/// The model this turn will actually run on, and what it can do.
pub(super) struct ModelChoice {
    pub(super) underpowered: bool,
    pub(super) http: crate::core::egress::Client,
    pub(super) prov: Option<&'static crate::core::providers::Provider>,
    pub(super) api_model: String,
    pub(super) profile: crate::core::model_profile::ModelProfile,
    pub(super) model_can_think: bool,
}

pub(super) async fn choose_model(
    app: &tauri::AppHandle,
    sink: &Option<EventSink>,
    manifest: &crate::core::personas::PersonaManifest,
    settings: &crate::core::settings::Settings,
    model: &String,
) -> Result<ModelChoice, String> {
    let emit = |kind: &str, text: String| turn_event(app, sink, kind, text);
    // An assistant whose work degrades badly on a small model, running on one.
    //
    // It is not forced onto a better model: that would either override the model the
    // user picked in the composer or push a local-only user's code to a cloud they
    // deliberately switched off. What it must not do is produce a confident report
    // that reads exactly like a thorough one — a security triage done by a small
    // model invents findings with the same fluency it finds real ones, and the
    // reader cannot tell. So it is told to say so, which is the rule the Coverage
    // note already follows: a check that ran weakly must not look like one that ran
    // well.
    let underpowered = manifest.needs_capable_model
        && !crate::core::providers::cloud_model_ready(&model)
        && crate::core::providers::resolve(&model).is_none();

    // The egress client, needed by the profile below and by MCP after it.
    let http = crate::core::egress::client("mcp");

    // Route by the model's provider: an OpenAI-compatible endpoint (covers Ollama
    // + OpenAI/Gemini/DeepSeek/xAI/Groq/Mistral/…) or Claude's native API.
    let prov = crate::core::providers::resolve(&model);
    let mut api_model = crate::core::providers::model_name(&model).to_string();
    // Local models: the configured id may not be pulled. Resolve against what
    // Ollama actually has rather than 404ing the turn.
    // The tag listing is fetched once here and then handed to the profile below —
    // a second `/api/tags` inside the step loop was both redundant and a place for a
    // capability to silently resolve `false`.
    let mut installed_infos: Option<Vec<crate::core::ollama::ModelInfo>> = None;
    if prov.is_none() {
        if let Ok(installed) = crate::core::ollama::list_model_infos(&settings).await {
            if !installed.iter().any(|m| m.name == api_model) {
                match crate::core::ollama::resolve_chat_model_info(&installed, &api_model) {
                    Some(m) => {
                        // Say it. Substituting silently is how a 30B model with no tool
                        // support ran every turn on a 16 GB laptop for weeks while the UI
                        // kept showing the configured model's name — the turns were slow
                        // and kept failing, and nothing connected that to the model.
                        if m != api_model {
                            emit(
                                "text",
                                format!("“{api_model}” isn't installed — running “{m}” instead. Pick a model in Settings to change this."),
                            );
                        }
                        api_model = m;
                    }
                    None => return Err(format!(
                        "'{api_model}' isn't installed and no other chat model is available. Pull one, e.g. `ollama pull llama3.2`."
                    )),
                }
            }
            installed_infos = Some(installed);
        }
    }

    // ---- What this model can actually do ---------------------------------------
    // HARNESS-PLAN H1: every question the loop used to answer by branching on vendor
    // identity — can it think, how big is its window, does it report usage — is a
    // property of the model, and is discoverable. Resolved once, after any
    // substitution above, so it describes the model that will actually run.
    let profile = crate::core::model_profile::resolve(
        &settings,
        // The bare name for a local model, because that is what was substituted; the
        // full `provider:model` id for everything else, because the prefix is how the
        // provider is found.
        if prov.is_none() { &api_model } else { &model },
        &http,
        installed_infos.as_deref(),
    )
    .await;
    if !profile.can_run_a_turn() {
        // Found before the user waits, rather than after a turn that could never have
        // completed: this loop IS tool calls, so a chat-only model cannot run one.
        return Err(format!(
            "“{api_model}” can't call tools, so it can't run a turn here. Pick a tool-capable model in the model menu."
        ));
    }
    let model_can_think = profile.can_think();
    Ok(ModelChoice { underpowered, http, prov, api_model, profile, model_can_think })
}
