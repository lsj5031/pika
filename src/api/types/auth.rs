//! Authentication API types.

use serde::{Deserialize, Serialize};

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