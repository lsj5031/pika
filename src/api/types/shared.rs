//! Shared types used across multiple API domains.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use serde::{Deserialize, Serialize};

/// Generic paged response wrapper
#[derive(Debug, Serialize)]
pub struct PagedResponse<T> {
    pub data: Vec<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<usize>,
}

/// Query parameters for paged sessions
#[derive(Debug, Deserialize)]
pub struct PagedSessionsQuery {
    pub limit: Option<usize>,
    pub cursor: Option<String>,
    pub q: Option<String>,
}

/// Query parameters for paged session messages
#[derive(Debug, Deserialize)]
pub struct SessionMessagesPagedQuery {
    pub limit: Option<usize>,
    pub before: Option<String>,
}

/// Query parameters for session messages
#[derive(Debug, Deserialize)]
pub struct SessionMessagesQuery {
    pub limit: Option<usize>,
    pub direction: Option<String>,
}

/// Request to look up multiple sessions by ID
#[derive(Debug, Deserialize)]
pub struct SessionsLookupRequest {
    pub ids: Vec<String>,
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

/// API error response
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
}

impl IntoResponse for ErrorResponse {
    fn into_response(self) -> Response {
        let status = match self.error.as_str() {
            "NOT_FOUND" | "PROJECT_NOT_FOUND" | "SESSION_NOT_FOUND" => StatusCode::NOT_FOUND,
            "BAD_REQUEST" | "INVALID_PATH" | "PROJECT_EXISTS" | "NOT_RUNNING"
            | "VALIDATION_ERROR" | "SESSION_CREATE_FAILED" => StatusCode::BAD_REQUEST,
            "UNAUTHORIZED" => StatusCode::UNAUTHORIZED,
            "SESSION_STOPPED" => StatusCode::GONE,
            "TOO_MANY_REQUESTS" => StatusCode::TOO_MANY_REQUESTS,
            "PAYLOAD_TOO_LARGE" => StatusCode::PAYLOAD_TOO_LARGE,
            "CONFIG_SAVE_FAILED" | "INTERNAL_ERROR" => StatusCode::INTERNAL_SERVER_ERROR,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, Json(self)).into_response()
    }
}