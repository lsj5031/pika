use axum::{Json, extract::State};

use crate::AppState;

use super::types::{ErrorResponse, ModelInfo, PikaSettingsResponse, UpdatePikaSettingsRequest};

/// Kill all running pika-agent processes so they restart with new settings.
async fn kill_all_running_processes(state: &AppState) {
    let process_ids = {
        let pm = state.process_manager.lock().await;
        pm.list()
    };

    if process_ids.is_empty() {
        return;
    }

    let mut pm = state.process_manager.lock().await;
    for pid in process_ids {
        if let Err(e) = pm.kill(&pid).await {
            tracing::warn!(process_id = %pid, error = %e, "Failed to kill process during settings update");
        }
    }
}

/// GET /api/settings - get PI settings
pub async fn get_pi_settings(
    State(_state): State<AppState>,
) -> Result<Json<PikaSettingsResponse>, ErrorResponse> {

    let pi_dir = crate::sessions::pi_agent_dir();

    let settings_path = pi_dir.join("settings.json");
    let models_path = pi_dir.join("models.json");

    // Read settings
    let settings = if tokio::fs::try_exists(&settings_path).await.unwrap_or(false) {
        tokio::fs::read_to_string(&settings_path)
            .await
            .ok()
            .and_then(|content| serde_json::from_str(&content).ok())
            .unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    // Read models
    let models = if tokio::fs::try_exists(&models_path).await.unwrap_or(false) {
        tokio::fs::read_to_string(&models_path)
            .await
            .ok()
            .and_then(|content| serde_json::from_str(&content).ok())
            .and_then(|value: serde_json::Value| {
                value
                    .get("providers")
                    .and_then(|p| p.as_object())
                    .map(|providers| {
                        providers
                            .iter()
                            .flat_map(|(provider_name, provider_data)| {
                                provider_data
                                    .get("models")
                                    .and_then(|m| m.as_array())
                                    .unwrap_or(&vec![])
                                    .iter()
                                    .filter_map(|model| {
                                        Some(ModelInfo {
                                            id: model.get("id")?.as_str()?.to_string(),
                                            name: model.get("name")?.as_str()?.to_string(),
                                            provider: provider_name.clone(),
                                            context_window: model
                                                .get("contextWindow")
                                                .and_then(|c| c.as_u64())
                                                .map(|c| c as usize),
                                            max_tokens: model
                                                .get("maxTokens")
                                                .and_then(|m| m.as_u64())
                                                .map(|m| m as usize),
                                            reasoning: model
                                                .get("reasoning")
                                                .and_then(|r| r.as_bool())
                                                .unwrap_or(false),
                                        })
                                    })
                                    .collect::<Vec<_>>()
                            })
                            .collect::<Vec<_>>()
                    })
            })
            .unwrap_or_default()
    } else {
        vec![]
    };

    let response = PikaSettingsResponse {
        default_provider: settings
            .get("defaultProvider")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        default_model: settings
            .get("defaultModel")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        default_thinking_level: settings
            .get("defaultThinkingLevel")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        theme: settings
            .get("theme")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        hide_thinking_block: settings.get("hideThinkingBlock").and_then(|v| v.as_bool()),
        available_models: models,
    };

    Ok(Json(response))
}

/// POST /api/settings - update PI settings
pub async fn update_pi_settings(
    State(state): State<AppState>,
    Json(request): Json<UpdatePikaSettingsRequest>,
) -> Result<Json<serde_json::Value>, ErrorResponse> {

    let pi_dir = crate::sessions::pi_agent_dir();

    let settings_path = pi_dir.join("settings.json");

    // Read existing settings
    let mut settings = if tokio::fs::try_exists(&settings_path).await.unwrap_or(false) {
        tokio::fs::read_to_string(&settings_path)
            .await
            .ok()
            .and_then(|content| serde_json::from_str(&content).ok())
            .unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    // Detect model or provider change — requires process restart
    let model_changed = request.default_model.as_ref()
        .map(|m| settings.get("defaultModel").and_then(|v| v.as_str()) != Some(m))
        .unwrap_or(false);
    let provider_changed = request.default_provider.as_ref()
        .map(|p| settings.get("defaultProvider").and_then(|v| v.as_str()) != Some(p))
        .unwrap_or(false);
    let needs_restart = model_changed || provider_changed;

    // Update settings
    if let Some(default_model) = request.default_model {
        settings["defaultModel"] = serde_json::json!(default_model);
    }
    if let Some(default_thinking_level) = request.default_thinking_level {
        settings["defaultThinkingLevel"] = serde_json::json!(default_thinking_level);
    }
    if let Some(default_provider) = request.default_provider {
        settings["defaultProvider"] = serde_json::json!(default_provider);
    }
    if let Some(hide_thinking_block) = request.hide_thinking_block {
        settings["hideThinkingBlock"] = serde_json::json!(hide_thinking_block);
    }

    // Ensure directory exists
    if let Some(parent) = settings_path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| ErrorResponse {
                error: "INTERNAL_ERROR".to_string(),
                message: format!("Failed to create settings directory: {}", e),
            })?;
    }

    // Write settings
    tokio::fs::write(
        &settings_path,
        serde_json::to_string_pretty(&settings).unwrap(),
    )
    .await
    .map_err(|e| ErrorResponse {
        error: "INTERNAL_ERROR".to_string(),
        message: format!("Failed to write settings: {}", e),
    })?;

    // Kill running processes so they restart with new model/provider
    if needs_restart {
        kill_all_running_processes(&state).await;
    }

    Ok(Json(serde_json::json!({ "success": true })))
}
