//! Authentication primitives and middleware.
//!
//! Supports:
//! - Username/password validation at login endpoint
//! - Signed HttpOnly session cookie for protected routes

use axum::{
    body::Body,
    http::{HeaderMap, Request, Response, StatusCode, header},
    middleware::Next,
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use hmac::{Hmac, Mac};
use sha2::Sha256;

/// Session cookie name
pub const SESSION_COOKIE_NAME: &str = "pika_session";

type HmacSha256 = Hmac<Sha256>;

/// Credentials used for login validation
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

/// Signed session cookie manager.
#[derive(Debug, Clone)]
pub struct SessionCookieManager {
    signing_secret: Vec<u8>,
    ttl_seconds: u64,
    secure: bool,
    cookie_name: String,
}

impl SessionCookieManager {
    pub fn new(signing_secret: Vec<u8>, ttl_seconds: u64, secure: bool) -> Self {
        Self {
            signing_secret,
            ttl_seconds,
            secure,
            cookie_name: SESSION_COOKIE_NAME.to_string(),
        }
    }

    pub fn ttl_seconds(&self) -> u64 {
        self.ttl_seconds
    }

    /// Build signed Set-Cookie header value for authenticated session.
    pub fn issue_session_cookie(&self, username: &str) -> String {
        let now = now_unix_seconds();
        let expires_at = now.saturating_add(self.ttl_seconds);
        let payload = format!("{}:{}", username, expires_at);
        let signature = self.sign(payload.as_bytes());

        let payload_encoded = URL_SAFE_NO_PAD.encode(payload.as_bytes());
        let signature_encoded = URL_SAFE_NO_PAD.encode(signature);
        let token = format!("{}.{}", payload_encoded, signature_encoded);

        self.build_cookie_header(&token, self.ttl_seconds)
    }

    /// Build Set-Cookie header value that clears session cookie.
    pub fn clear_session_cookie(&self) -> String {
        self.build_cookie_header("", 0)
    }

    /// Validate session cookie from request headers.
    pub fn validate_session_from_headers(&self, headers: &HeaderMap) -> bool {
        let cookie_value = extract_cookie_value(headers, &self.cookie_name);
        match cookie_value {
            Some(token) => self.validate_session_token(&token),
            None => false,
        }
    }

    fn build_cookie_header(&self, value: &str, max_age_seconds: u64) -> String {
        let mut parts = vec![
            format!("{}={}", self.cookie_name, value),
            "HttpOnly".to_string(),
            "Path=/".to_string(),
            format!("Max-Age={}", max_age_seconds),
            "SameSite=Strict".to_string(),
        ];

        if self.secure {
            parts.push("Secure".to_string());
        }

        parts.join("; ")
    }

    fn validate_session_token(&self, token: &str) -> bool {
        let (payload_encoded, sig_encoded) = match token.split_once('.') {
            Some(parts) => parts,
            None => return false,
        };

        let payload = match URL_SAFE_NO_PAD.decode(payload_encoded) {
            Ok(p) => p,
            Err(_) => return false,
        };

        let signature = match URL_SAFE_NO_PAD.decode(sig_encoded) {
            Ok(s) => s,
            Err(_) => return false,
        };

        if !self.verify(&payload, &signature) {
            return false;
        }

        let payload_str = match String::from_utf8(payload) {
            Ok(s) => s,
            Err(_) => return false,
        };

        let (_username, expires_str) = match payload_str.rsplit_once(':') {
            Some(parts) => parts,
            None => return false,
        };

        let expires_at: u64 = match expires_str.parse() {
            Ok(ts) => ts,
            Err(_) => return false,
        };

        now_unix_seconds() <= expires_at
    }

    fn sign(&self, payload: &[u8]) -> Vec<u8> {
        let mut mac =
            HmacSha256::new_from_slice(&self.signing_secret).expect("HMAC key setup failed");
        mac.update(payload);
        mac.finalize().into_bytes().to_vec()
    }

    fn verify(&self, payload: &[u8], signature: &[u8]) -> bool {
        let mut mac =
            HmacSha256::new_from_slice(&self.signing_secret).expect("HMAC key setup failed");
        mac.update(payload);
        mac.verify_slice(signature).is_ok()
    }
}

/// Shared auth context used by middleware and handlers.
#[derive(Debug, Clone)]
pub struct AuthContext {
    pub credentials: AuthCredentials,
    pub auth_enabled: bool,
    pub session_cookie: SessionCookieManager,
}

impl AuthContext {
    pub fn new(
        credentials: AuthCredentials,
        auth_enabled: bool,
        session_cookie: SessionCookieManager,
    ) -> Self {
        Self {
            credentials,
            auth_enabled,
            session_cookie,
        }
    }
}

/// Constant-time string comparison to prevent timing attacks
fn constant_time_compare(a: &str, b: &str) -> bool {
    let a_bytes = a.as_bytes();
    let b_bytes = b.as_bytes();
    let max_len = a_bytes.len().max(b_bytes.len());

    let mut result = (a_bytes.len() ^ b_bytes.len()) as u8;
    for i in 0..max_len {
        let byte_a = a_bytes.get(i).copied().unwrap_or(0);
        let byte_b = b_bytes.get(i).copied().unwrap_or(0);
        result |= byte_a ^ byte_b;
    }
    result == 0
}

/// Create an unauthorized response without WWW-Authenticate header.
fn unauthorized_response() -> Response<Body> {
    Response::builder()
        .status(StatusCode::UNAUTHORIZED)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            r#"{"error":"UNAUTHORIZED","message":"Authentication required"}"#,
        ))
        .unwrap()
}

/// Parse one cookie value from `Cookie` header.
fn extract_cookie_value(headers: &HeaderMap, cookie_name: &str) -> Option<String> {
    let header_value = headers.get(header::COOKIE)?.to_str().ok()?;

    header_value.split(';').find_map(|pair| {
        let (key, value) = pair.trim().split_once('=')?;
        if key == cookie_name {
            Some(value.to_string())
        } else {
            None
        }
    })
}

/// Validate incoming request credentials using signed session cookie auth.
pub fn is_request_authenticated(headers: &HeaderMap, auth_context: &AuthContext) -> bool {
    if !auth_context.auth_enabled || !auth_context.credentials.is_enabled() {
        return true;
    }

    auth_context
        .session_cookie
        .validate_session_from_headers(headers)
}

/// HTTP auth middleware (session-cookie auth).
pub async fn auth_middleware(
    request: Request<Body>,
    next: Next,
    auth_context: AuthContext,
) -> Response<Body> {
    if is_request_authenticated(request.headers(), &auth_context) {
        next.run(request).await
    } else {
        unauthorized_response()
    }
}

fn now_unix_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
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
    fn test_request_authenticated_with_valid_cookie() {
        let manager = SessionCookieManager::new(b"secret".to_vec(), 3600, false);
        let cookie_header = manager.issue_session_cookie("admin");
        let cookie = cookie_header.split(';').next().unwrap().to_string();

        let mut headers = HeaderMap::new();
        headers.insert(header::COOKIE, cookie.parse().unwrap());

        let ctx = AuthContext::new(AuthCredentials::new("admin", "secret"), true, manager);

        assert!(is_request_authenticated(&headers, &ctx));
    }

    #[test]
    fn test_basic_auth_header_is_not_used_for_protected_routes() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            "Basic YWRtaW46c2VjcmV0".parse().unwrap(),
        );

        let ctx = AuthContext::new(
            AuthCredentials::new("admin", "secret"),
            true,
            SessionCookieManager::new(b"secret".to_vec(), 3600, false),
        );

        assert!(!is_request_authenticated(&headers, &ctx));
    }

    #[test]
    fn test_constant_time_compare() {
        assert!(constant_time_compare("hello", "hello"));
        assert!(!constant_time_compare("hello", "world"));
        assert!(!constant_time_compare("short", "longer"));
    }

    #[test]
    fn test_session_cookie_roundtrip() {
        let manager = SessionCookieManager::new(b"secret".to_vec(), 3600, false);
        let header = manager.issue_session_cookie("admin");

        let token = header
            .split(';')
            .next()
            .and_then(|part| part.split_once('='))
            .map(|(_, value)| value)
            .unwrap();

        assert!(manager.validate_session_token(token));
    }
}
