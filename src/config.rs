use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use thiserror::Error;

const DEFAULT_BIND_ADDRESS: &str = "127.0.0.1";
const DEFAULT_MAX_REQUEST_BODY_BYTES: usize = 25 * 1024 * 1024; // 25 MiB
const DEFAULT_MAX_PROMPT_CHARS: usize = 32_000;
const DEFAULT_MAX_IMAGES_PER_PROMPT: usize = 5;
const DEFAULT_MAX_IMAGE_BYTES: usize = 5 * 1024 * 1024; // 5 MiB per image
const DEFAULT_MAX_TOTAL_IMAGE_BYTES: usize = 20 * 1024 * 1024; // 20 MiB total
const DEFAULT_SESSION_TTL_SECONDS: u64 = 60 * 60 * 8; // 8 hours
const DEFAULT_LOGIN_RATE_LIMIT_PER_MINUTE: u32 = 20;
const DEFAULT_WS_RATE_LIMIT_PER_MINUTE: u32 = 60;
const MIN_SESSION_SECRET_BYTES: usize = 32;

fn default_bind_address() -> String {
    DEFAULT_BIND_ADDRESS.to_string()
}

fn default_max_request_body_bytes() -> usize {
    DEFAULT_MAX_REQUEST_BODY_BYTES
}

fn default_max_prompt_chars() -> usize {
    DEFAULT_MAX_PROMPT_CHARS
}

fn default_max_images_per_prompt() -> usize {
    DEFAULT_MAX_IMAGES_PER_PROMPT
}

fn default_max_image_bytes() -> usize {
    DEFAULT_MAX_IMAGE_BYTES
}

fn default_max_total_image_bytes() -> usize {
    DEFAULT_MAX_TOTAL_IMAGE_BYTES
}

fn default_allowed_image_mime_types() -> Vec<String> {
    vec![
        "image/png".to_string(),
        "image/jpeg".to_string(),
        "image/webp".to_string(),
        "image/gif".to_string(),
    ]
}

fn default_session_ttl_seconds() -> u64 {
    DEFAULT_SESSION_TTL_SECONDS
}

fn default_login_rate_limit_per_minute() -> u32 {
    DEFAULT_LOGIN_RATE_LIMIT_PER_MINUTE
}

fn default_ws_connect_rate_limit_per_minute() -> u32 {
    DEFAULT_WS_RATE_LIMIT_PER_MINUTE
}

/// Project configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    /// List of project root paths to scan for sessions
    #[serde(default)]
    pub project_root_paths: Vec<PathBuf>,

    /// Optional allowlist for project/session path creation.
    /// If empty, any path is allowed.
    #[serde(default)]
    pub allowed_project_roots: Vec<PathBuf>,

    /// Disable auth requirement (debug only, can be set via AUTH_DISABLE env var)
    #[serde(default)]
    pub debug_disable_auth: bool,

    /// Bind address for HTTP server (can be overridden via BIND_ADDRESS env var)
    #[serde(default = "default_bind_address")]
    pub bind_address: String,

    /// Allowed CORS origins. If empty, localhost-only defaults are used.
    #[serde(default)]
    pub cors_allowed_origins: Vec<String>,

    /// Explicit override for insecure remote mode (can be set via ALLOW_INSECURE_REMOTE)
    #[serde(default)]
    pub allow_insecure_remote: bool,

    /// Global maximum request body size
    #[serde(default = "default_max_request_body_bytes")]
    pub max_request_body_bytes: usize,

    /// Maximum number of prompt characters
    #[serde(default = "default_max_prompt_chars")]
    pub max_prompt_chars: usize,

    /// Maximum number of images attached to a prompt
    #[serde(default = "default_max_images_per_prompt")]
    pub max_images_per_prompt: usize,

    /// Maximum decoded size for a single image attachment
    #[serde(default = "default_max_image_bytes")]
    pub max_image_bytes: usize,

    /// Maximum decoded total size of all image attachments in a prompt
    #[serde(default = "default_max_total_image_bytes")]
    pub max_total_image_bytes: usize,

    /// Allowed MIME types for prompt image uploads
    #[serde(default = "default_allowed_image_mime_types")]
    pub allowed_image_mime_types: Vec<String>,

    /// Session cookie TTL in seconds
    #[serde(default = "default_session_ttl_seconds")]
    pub session_ttl_seconds: u64,

    /// Force secure attribute on session cookie.
    /// If `None`, secure defaults to `true`.
    #[serde(default)]
    pub session_cookie_secure: Option<bool>,

    /// Login attempts per minute per IP
    #[serde(default = "default_login_rate_limit_per_minute")]
    pub login_rate_limit_per_minute: u32,

    /// WebSocket connect attempts per minute per IP
    #[serde(default = "default_ws_connect_rate_limit_per_minute")]
    pub ws_connect_rate_limit_per_minute: u32,

    /// Trusted reverse-proxy CIDRs for honoring forwarded client IP headers.
    /// Requests from peers outside these CIDRs will ignore X-Forwarded-For/X-Real-IP.
    #[serde(default)]
    pub trusted_proxy_cidrs: Vec<String>,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            project_root_paths: Vec::new(),
            allowed_project_roots: Vec::new(),
            debug_disable_auth: false,
            bind_address: default_bind_address(),
            cors_allowed_origins: Vec::new(),
            allow_insecure_remote: false,
            max_request_body_bytes: default_max_request_body_bytes(),
            max_prompt_chars: default_max_prompt_chars(),
            max_images_per_prompt: default_max_images_per_prompt(),
            max_image_bytes: default_max_image_bytes(),
            max_total_image_bytes: default_max_total_image_bytes(),
            allowed_image_mime_types: default_allowed_image_mime_types(),
            session_ttl_seconds: default_session_ttl_seconds(),
            session_cookie_secure: None,
            login_rate_limit_per_minute: default_login_rate_limit_per_minute(),
            ws_connect_rate_limit_per_minute: default_ws_connect_rate_limit_per_minute(),
            trusted_proxy_cidrs: Vec::new(),
        }
    }
}

impl ProjectConfig {
    /// Get the auth username from environment.
    /// Credentials are intentionally environment-only.
    pub fn get_auth_username(&self) -> Option<String> {
        std::env::var("AUTH_USERNAME")
            .ok()
            .filter(|s| !s.is_empty())
    }

    /// Get the auth password from environment.
    /// Credentials are intentionally environment-only.
    pub fn get_auth_password(&self) -> Option<String> {
        std::env::var("AUTH_PASSWORD")
            .ok()
            .filter(|s| !s.is_empty())
    }

    /// Get optional session-signing secret from environment.
    pub fn get_session_secret(&self) -> Option<String> {
        std::env::var("AUTH_SESSION_SECRET")
            .ok()
            .filter(|s| !s.trim().is_empty())
    }

    /// Validate session secret strength.
    pub fn validate_session_secret_strength(secret: &str) -> Result<(), String> {
        if secret.as_bytes().len() < MIN_SESSION_SECRET_BYTES {
            return Err(format!(
                "AUTH_SESSION_SECRET must be at least {} bytes",
                MIN_SESSION_SECRET_BYTES
            ));
        }

        Ok(())
    }

    /// Check if authentication is explicitly disabled (debug mode)
    pub fn is_auth_disabled(&self) -> bool {
        match std::env::var("AUTH_DISABLE")
            .ok()
            .as_deref()
            .map(parse_bool_value)
        {
            Some(Some(value)) => value,
            Some(None) | None => self.debug_disable_auth,
        }
    }

    /// Check if authentication is enabled
    pub fn is_auth_enabled(&self) -> bool {
        if self.is_auth_disabled() {
            return false;
        }

        self.get_auth_username().is_some() && self.get_auth_password().is_some()
    }

    /// Effective bind address (config value overridden by env)
    pub fn get_bind_address(&self) -> String {
        std::env::var("BIND_ADDRESS")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| self.bind_address.clone())
    }

    /// Whether explicit insecure remote mode is allowed
    pub fn allow_insecure_remote_mode(&self) -> bool {
        match std::env::var("ALLOW_INSECURE_REMOTE")
            .ok()
            .as_deref()
            .map(parse_bool_value)
        {
            Some(Some(value)) => value,
            Some(None) | None => self.allow_insecure_remote,
        }
    }

    /// Effective list of allowed CORS origins.
    /// Env var format: comma-separated URLs.
    pub fn get_allowed_cors_origins(&self) -> Vec<String> {
        if let Ok(origins) = std::env::var("CORS_ALLOWED_ORIGINS") {
            let parsed: Vec<String> = origins
                .split(',')
                .map(|origin| origin.trim().to_string())
                .filter(|origin| !origin.is_empty())
                .collect();

            if !parsed.is_empty() {
                return parsed;
            }
        }

        self.cors_allowed_origins.clone()
    }

    /// Effective allowed project roots.
    /// Env var format: colon-separated absolute paths.
    pub fn get_allowed_project_roots(&self) -> Vec<PathBuf> {
        if let Ok(roots) = std::env::var("ALLOWED_PROJECT_ROOTS") {
            let parsed: Vec<PathBuf> = roots
                .split(':')
                .map(str::trim)
                .filter(|path| !path.is_empty())
                .map(PathBuf::from)
                .collect();

            if !parsed.is_empty() {
                return parsed;
            }
        }

        self.allowed_project_roots.clone()
    }

    /// Effective trusted proxy CIDRs.
    /// Env var format: comma-separated CIDRs or single IPs.
    pub fn get_trusted_proxy_cidrs(&self) -> Vec<String> {
        if let Ok(cidrs) = std::env::var("TRUSTED_PROXY_CIDRS") {
            let parsed: Vec<String> = cidrs
                .split(',')
                .map(str::trim)
                .filter(|cidr| !cidr.is_empty())
                .map(ToString::to_string)
                .collect();

            if !parsed.is_empty() {
                return parsed;
            }
        }

        self.trusted_proxy_cidrs.clone()
    }

    /// Resolve whether session cookie should set Secure attribute.
    pub fn session_cookie_secure(&self) -> bool {
        self.session_cookie_secure.unwrap_or(true)
    }

    /// Check whether a candidate path is allowed by allowed_project_roots.
    /// If no roots are configured, all paths are allowed.
    pub fn is_path_allowed(&self, candidate_path: &Path) -> bool {
        let roots = self.get_allowed_project_roots();
        if roots.is_empty() {
            return true;
        }

        let canonical_candidate = match candidate_path.canonicalize() {
            Ok(path) => path,
            Err(_) => return false,
        };

        roots.into_iter().any(|root| {
            root.canonicalize()
                .map(|canonical_root| canonical_candidate.starts_with(&canonical_root))
                .unwrap_or(false)
        })
    }
}

fn parse_bool_value(value: &str) -> Option<bool> {
    match value.trim().to_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

impl ProjectConfig {
    /// Load configuration from a TOML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let path = path.as_ref();

        // If file doesn't exist, return default config
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(path).map_err(|e| ConfigError::IoError {
            path: path.to_path_buf(),
            source: e,
        })?;

        let config: Self = toml::from_str(&content).map_err(|e| ConfigError::ParseError {
            path: path.to_path_buf(),
            source: e,
        })?;

        Ok(config)
    }

    /// Validate that all configured paths exist
    pub fn validate(&self) -> Result<(), ConfigError> {
        for path in &self.project_root_paths {
            if !path.exists() {
                return Err(ConfigError::PathNotFound { path: path.clone() });
            }
        }

        for path in self.get_allowed_project_roots() {
            if !path.exists() {
                return Err(ConfigError::PathNotFound { path });
            }
        }

        Ok(())
    }

    /// Save configuration to a TOML file
    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), ConfigError> {
        let path = path.as_ref();

        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent()
            && !parent.exists()
        {
            fs::create_dir_all(parent).map_err(|e| ConfigError::IoError {
                path: parent.to_path_buf(),
                source: e,
            })?;
        }

        let content =
            toml::to_string_pretty(self).map_err(|e| ConfigError::SerializeError { source: e })?;

        fs::write(path, content).map_err(|e| ConfigError::IoError {
            path: path.to_path_buf(),
            source: e,
        })?;

        Ok(())
    }
}

/// Configuration errors
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Failed to read config file {path}: {source}")]
    IoError { path: PathBuf, source: io::Error },
    #[error("Failed to parse config file {path}: {source}")]
    ParseError {
        path: PathBuf,
        source: toml::de::Error,
    },
    #[error("Failed to serialize config: {source}")]
    SerializeError { source: toml::ser::Error },
    #[error("Configured path does not exist: {path}")]
    PathNotFound { path: PathBuf },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ProjectConfig::default();
        assert!(config.project_root_paths.is_empty());
        assert_eq!(config.bind_address, "127.0.0.1");
    }

    #[test]
    fn test_auth_disabled_via_config() {
        let config = ProjectConfig {
            debug_disable_auth: true,
            ..ProjectConfig::default()
        };
        assert!(!config.is_auth_enabled());
    }

    #[test]
    fn test_validate_empty_config() {
        let config = ProjectConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_invalid_path() {
        let mut config = ProjectConfig::default();
        config
            .project_root_paths
            .push(PathBuf::from("/nonexistent/path/that/does/not/exist"));
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_session_cookie_secure_defaults_to_true() {
        let config = ProjectConfig::default();
        assert!(config.session_cookie_secure());
    }

    #[test]
    fn test_session_cookie_secure_explicit_override() {
        let config = ProjectConfig {
            session_cookie_secure: Some(false),
            ..ProjectConfig::default()
        };
        assert!(!config.session_cookie_secure());
    }

    #[test]
    fn test_validate_session_secret_strength() {
        assert!(
            ProjectConfig::validate_session_secret_strength("12345678901234567890123456789012")
                .is_ok()
        );
        assert!(ProjectConfig::validate_session_secret_strength("too-short").is_err());
    }
}
