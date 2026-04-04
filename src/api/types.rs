use axum::{
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Project information response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectResponse {
    /// Unique project ID (based on path hash for now)
    pub id: String,
    /// Project root path
    pub path: PathBuf,
    /// Project name (extracted from path)
    pub name: String,
    /// Number of sessions found in this project
    pub session_count: usize,
}

/// Session details response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionResponse {
    /// Unique session identifier
    pub id: String,
    /// Session name
    pub name: String,
    /// Project ID containing this session
    pub project_id: String,
    /// Project path containing this session
    pub project_path: PathBuf,
    /// Session creation timestamp
    pub created_at: String,
    /// Whether the session is currently active
    pub is_active: bool,
}

/// Message in a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageResponse {
    /// Message role ("user" or "assistant")
    pub role: String,
    /// Message content
    pub content: String,
    /// Message timestamp (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
    /// Image attachments (for user messages with images)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<ImageAttachmentResponse>>,
}

/// Image attachment in a message response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageAttachmentResponse {
    /// Unique image ID
    pub id: String,
    /// Original filename
    pub filename: String,
    /// MIME type
    pub content_type: String,
    /// Image size in bytes
    pub size: usize,
    /// URL to access the image (data URL format)
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct SessionMessagesQuery {
    pub limit: Option<usize>,
    pub direction: Option<String>,
}

/// Image attachment in a prompt request
#[derive(Debug, Deserialize)]
pub struct ImageAttachment {
    /// Original filename
    pub filename: String,
    /// MIME type (e.g., "image/png", "image/jpeg")
    pub content_type: String,
    /// Base64 encoded image data (without data URL prefix)
    pub data: String,
}

/// Request to send a prompt to a session
#[derive(Debug, Deserialize)]
pub struct PromptRequest {
    /// The prompt text to send
    pub prompt: String,
    /// Optional image attachments (base64 encoded)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<ImageAttachment>>,
}

/// Response when starting a session
#[derive(Debug, Serialize)]
pub struct StartSessionResponse {
    /// The process ID that was started (or already running)
    pub process_id: String,
    /// Whether the process was newly spawned (false if already running)
    pub newly_spawned: bool,
}

/// Response for session status
#[derive(Debug, Serialize)]
pub struct SessionStatusResponse {
    /// The session ID
    pub session_id: String,
    /// Whether the session process is currently running
    pub is_running: bool,
    /// The process ID (if running)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process_id: Option<String>,
}

/// Response when stopping a session
#[derive(Debug, Serialize)]
pub struct StopSessionResponse {
    /// The session ID that was stopped
    pub session_id: String,
    /// The process ID that was killed
    pub process_id: Option<String>,
    /// Whether the process was running and was stopped
    pub was_running: bool,
}

/// Auth status response
#[derive(Debug, Serialize)]
pub struct AuthStatusResponse {
    /// Whether auth is enabled
    pub enabled: bool,
    /// Whether the current request is already authenticated (session cookie)
    pub authenticated: bool,
}

/// Login request body
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// Login response body
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub success: bool,
    pub expires_in_seconds: u64,
}

/// API error response
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
}

impl IntoResponse for ErrorResponse {
    fn into_response(self) -> Response {
        let status = match self.error.as_str() {
            "NOT_FOUND" | "PROJECT_NOT_FOUND" => StatusCode::NOT_FOUND,
            "BAD_REQUEST" | "INVALID_PATH" | "PROJECT_EXISTS" | "NOT_RUNNING"
            | "VALIDATION_ERROR" => StatusCode::BAD_REQUEST,
            "UNAUTHORIZED" => StatusCode::UNAUTHORIZED,
            "SESSION_STOPPED" => StatusCode::GONE,
            "TOO_MANY_REQUESTS" => StatusCode::TOO_MANY_REQUESTS,
            "PAYLOAD_TOO_LARGE" => StatusCode::PAYLOAD_TOO_LARGE,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, Json(self)).into_response()
    }
}

/// Generic paged response wrapper
#[derive(Debug, Serialize)]
pub struct PagedResponse<T> {
    pub data: Vec<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct PagedSessionsQuery {
    pub limit: Option<usize>,
    pub cursor: Option<String>,
    pub q: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SessionMessagesPagedQuery {
    pub limit: Option<usize>,
    pub before: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SessionsLookupRequest {
    pub ids: Vec<String>,
}

/// Request to add a new project
#[derive(Debug, Deserialize)]
pub struct AddProjectRequest {
    /// Path to the project root
    pub path: String,
}

/// Response when adding a project
#[derive(Debug, Serialize)]
pub struct AddProjectResponse {
    /// The project ID
    pub id: String,
    /// The project name
    pub name: String,
    /// The project path
    pub path: PathBuf,
}

#[derive(Debug, Deserialize)]
pub struct SetThinkingLevelRequest {
    pub level: String,
}

/// Request to create a new session in a project
#[derive(Debug, Deserialize)]
pub struct CreateSessionInProjectRequest {
    /// Optional session name (defaults to timestamp if not provided)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Response when creating a new session in a project
#[derive(Debug, Serialize)]
pub struct CreateSessionInProjectResponse {
    /// The newly created session ID
    pub session_id: String,
    /// The session name
    pub name: String,
    /// The project ID where the session was created
    pub project_id: String,
    /// The project path where the session was created
    pub project_path: PathBuf,
    /// The session creation timestamp
    pub created_at: String,
    /// Whether the process was newly spawned
    pub newly_spawned: bool,
    /// The process ID (if spawned)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process_id: Option<String>,
}

/// Request to create a standalone session in any folder
#[derive(Debug, Deserialize)]
pub struct CreateStandaloneSessionRequest {
    /// Path to the folder where the session should be created
    pub path: String,
    /// Optional session name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Response when creating a standalone session
#[derive(Debug, Serialize)]
pub struct CreateStandaloneSessionResponse {
    /// The newly created session ID
    pub session_id: String,
    /// The session name
    pub name: String,
    /// The path where the session was created
    pub path: PathBuf,
    /// The session creation timestamp
    pub created_at: String,
}

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
