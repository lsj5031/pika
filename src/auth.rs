//! HTTP Basic Authentication middleware for axum
//!
//! This module provides middleware for protecting routes with HTTP Basic Auth.
//! Credentials are configured via config.toml or environment variables.

use axum::{
    body::Body,
    http::{header, Request, Response, StatusCode},
    middleware::Next,
};
use base64::{engine::general_purpose::STANDARD, Engine};

/// Credentials for HTTP Basic Auth
#[derive(Debug, Clone)]
pub struct AuthCredentials {
    pub username: String,
    pub password: String,
}

impl AuthCredentials {
    /// Create new credentials
    pub fn new(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            username: username.into(),
            password: password.into(),
        }
    }

    /// Check if authentication is enabled (both username and password are set)
    pub fn is_enabled(&self) -> bool {
        !self.username.is_empty() && !self.password.is_empty()
    }

    /// Validate credentials against the stored ones
    pub fn validate(&self, username: &str, password: &str) -> bool {
        // Use constant-time comparison to prevent timing attacks
        let username_matches = constant_time_compare(&self.username, username);
        let password_matches = constant_time_compare(&self.password, password);
        username_matches && password_matches
    }
}

/// Constant-time string comparison to prevent timing attacks
fn constant_time_compare(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut result = 0u8;
    for (byte_a, byte_b) in a.bytes().zip(b.bytes()) {
        result |= byte_a ^ byte_b;
    }
    result == 0
}

/// Extract and decode Basic Auth credentials from Authorization header
fn extract_basic_auth(auth_header: &str) -> Option<(String, String)> {
    // Must start with "Basic "
    let encoded = auth_header.strip_prefix("Basic ")?;

    // Decode base64
    let decoded = STANDARD.decode(encoded).ok()?;
    let decoded_str = String::from_utf8(decoded).ok()?;

    // Split on first colon (password may contain colons)
    let (username, password) = decoded_str.split_once(':')?;
    Some((username.to_string(), password.to_string()))
}

/// Create an unauthorized response with WWW-Authenticate header
fn unauthorized_response() -> Response<Body> {
    Response::builder()
        .status(StatusCode::UNAUTHORIZED)
        .header(header::WWW_AUTHENTICATE, r#"Basic realm="PI Agent Manager""#)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"error":"Unauthorized","message":"Authentication required"}"#))
        .unwrap()
}

/// HTTP Basic Auth middleware
///
/// This middleware checks the Authorization header for valid Basic Auth credentials.
/// If credentials are missing or invalid, it returns 401 Unauthorized with
/// WWW-Authenticate header to trigger the browser's login prompt.
pub async fn basic_auth_middleware(
    request: Request<Body>,
    next: Next,
    credentials: AuthCredentials,
) -> Response<Body> {
    // Skip auth if not enabled
    if !credentials.is_enabled() {
        return next.run(request).await;
    }

    // Get Authorization header
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok());

    // Extract and validate credentials
    match auth_header {
        Some(auth) => match extract_basic_auth(auth) {
            Some((username, password)) => {
                if credentials.validate(&username, &password) {
                    // Valid credentials - proceed
                    next.run(request).await
                } else {
                    // Invalid credentials
                    unauthorized_response()
                }
            }
            None => {
                // Malformed Authorization header
                unauthorized_response()
            }
        },
        None => {
            // No Authorization header
            unauthorized_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_credentials_enabled() {
        let creds = AuthCredentials::new("user", "pass");
        assert!(creds.is_enabled());

        let empty = AuthCredentials::new("", "");
        assert!(!empty.is_enabled());

        let partial = AuthCredentials::new("user", "");
        assert!(!partial.is_enabled());
    }

    #[test]
    fn test_credentials_validate() {
        let creds = AuthCredentials::new("admin", "secret");
        assert!(creds.validate("admin", "secret"));
        assert!(!creds.validate("admin", "wrong"));
        assert!(!creds.validate("wrong", "secret"));
    }

    #[test]
    fn test_extract_basic_auth() {
        // Valid header: "admin:password" encoded
        let (user, pass) = extract_basic_auth("Basic YWRtaW46cGFzc3dvcmQ=").unwrap();
        assert_eq!(user, "admin");
        assert_eq!(pass, "password");

        // Password with colon: "admin:pass:word"
        let (user, pass) = extract_basic_auth("Basic YWRtaW46cGFzczp3b3Jk").unwrap();
        assert_eq!(user, "admin");
        assert_eq!(pass, "pass:word");

        // Invalid header
        assert!(extract_basic_auth("Bearer token").is_none());
        assert!(extract_basic_auth("Basic invalid!!!").is_none());
    }

    #[test]
    fn test_constant_time_compare() {
        assert!(constant_time_compare("hello", "hello"));
        assert!(!constant_time_compare("hello", "world"));
        assert!(!constant_time_compare("short", "longer"));
    }
}
