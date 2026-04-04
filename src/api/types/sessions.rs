//! Session API types.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::ImageAttachmentResponse;

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

/// Request to send a prompt to a session
#[derive(Debug, Deserialize)]
pub struct PromptRequest {
    /// The prompt text to send
    pub prompt: String,
    /// Optional image attachments (base64 encoded)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<super::ImageAttachment>>,
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

/// Request to set thinking level
#[derive(Debug, Deserialize)]
pub struct SetThinkingLevelRequest {
    pub level: String,
}