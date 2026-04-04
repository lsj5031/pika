//! Settings API types.

use serde::{Deserialize, Serialize};

/// PI settings response
#[derive(Debug, Serialize)]
pub struct PikaSettingsResponse {
    /// Default provider
    #[serde(rename = "defaultProvider")]
    pub default_provider: Option<String>,
    /// Default model
    #[serde(rename = "defaultModel")]
    pub default_model: Option<String>,
    /// Default thinking level
    #[serde(rename = "defaultThinkingLevel")]
    pub default_thinking_level: Option<String>,
    /// Theme
    #[serde(rename = "theme")]
    pub theme: Option<String>,
    /// Hide thinking block
    #[serde(rename = "hideThinkingBlock")]
    pub hide_thinking_block: Option<bool>,
    /// Available models
    #[serde(rename = "availableModels")]
    pub available_models: Vec<ModelInfo>,
}

/// Model information
#[derive(Debug, Serialize)]
pub struct ModelInfo {
    /// Model ID
    pub id: String,
    /// Model name
    pub name: String,
    /// Provider
    pub provider: String,
    /// Context window
    pub context_window: Option<usize>,
    /// Max tokens
    pub max_tokens: Option<usize>,
    /// Reasoning capability
    pub reasoning: bool,
}

/// Request to update PI settings
#[derive(Debug, Deserialize)]
pub struct UpdatePikaSettingsRequest {
    /// Default model
    #[serde(rename = "defaultModel", skip_serializing_if = "Option::is_none")]
    pub default_model: Option<String>,
    /// Default thinking level
    #[serde(
        rename = "defaultThinkingLevel",
        skip_serializing_if = "Option::is_none"
    )]
    pub default_thinking_level: Option<String>,
    /// Default provider
    #[serde(rename = "defaultProvider", skip_serializing_if = "Option::is_none")]
    pub default_provider: Option<String>,
    /// Hide thinking block
    #[serde(rename = "hideThinkingBlock", skip_serializing_if = "Option::is_none")]
    pub hide_thinking_block: Option<bool>,
}