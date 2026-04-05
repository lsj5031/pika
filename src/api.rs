//! API module - provides HTTP endpoints for the agent manager
//! 
//! This module is organized into sub-modules:
//! - auth: Authentication endpoints (login, logout, status)
//! - projects: Project management endpoints
//! - sessions: Session management and messaging endpoints
//! - settings: Pika settings endpoints
//! - types: Shared request/response types
//! - routes: Router configuration

mod auth;
mod projects;
mod routes;
mod sessions;
mod settings;
pub mod types;

pub use routes::{create_api_router, create_auth_router};

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::config::ProjectConfig;

/// API state shared across all handlers
#[derive(Clone)]
pub struct ApiState {
    pub config: Arc<RwLock<ProjectConfig>>,
    pub config_path: PathBuf,
}

impl ApiState {
    pub fn new(config: ProjectConfig, config_path: PathBuf) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            config_path,
        }
    }
}

// Re-export handlers for convenience (routes.rs uses these)
pub use auth::{get_auth_status, login, logout};
pub use projects::{add_project, get_project_sessions, get_projects, remove_project};
pub use sessions::{
    create_session_in_project, create_standalone_session, cycle_thinking_level, get_session,
    get_session_messages, get_session_messages_paged, get_session_status, get_sessions,
    get_sessions_paged, get_project_sessions_paged, lookup_sessions, send_prompt_to_session,
    set_thinking_level, start_session, stop_session,
};



/// Pagination constants
pub const DEFAULT_PAGE_LIMIT: usize = 50;
pub const MAX_PAGE_LIMIT: usize = 200;

/// Generate a project ID from path (simple hash-based approach)
pub fn project_id_from_path(path: &PathBuf) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

/// Extract project name from path
pub fn project_name_from_path(path: &std::path::Path) -> String {
    path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown")
        .to_string()
}

/// Resolve a canonical path from user input (expanding ~ and making absolute)
pub fn resolve_canonical_path(input_path: &str) -> Result<PathBuf, types::ErrorResponse> {
    let expanded_path = if let Some(stripped) = input_path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            home.join(stripped)
        } else {
            PathBuf::from(input_path)
        }
    } else {
        PathBuf::from(input_path)
    };

    let absolute_path = if expanded_path.is_absolute() {
        expanded_path
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(&expanded_path)
    };

    absolute_path.canonicalize().map_err(|_| types::ErrorResponse {
        error: "INVALID_PATH".to_string(),
        message: format!("Cannot canonicalize path: {}", absolute_path.display()),
    })
}

/// Enforce project root path policy - ensures path is within allowed roots
pub async fn enforce_project_root_policy(
    state: &crate::AppState,
    candidate_path: &std::path::Path,
) -> Result<(), types::ErrorResponse> {
    let config = state.api_state.config.read().await;

    if config.is_path_allowed(candidate_path) {
        Ok(())
    } else {
        Err(types::ErrorResponse {
            error: "INVALID_PATH".to_string(),
            message: format!(
                "Path '{}' is outside configured allowed project roots",
                candidate_path.display()
            ),
        })
    }
}

/// Find a session by ID in the session index
pub async fn find_session(
    state: &crate::AppState,
    session_id: &str,
) -> Option<crate::sessions::SessionInfo> {
    let index = state.session_index.read().await;
    index.get(session_id).cloned()
}

/// Merge stored user prompts with pi-agent messages based on timestamps
pub fn merge_messages(
    stored_prompts: Vec<crate::sessions::SessionMessage>,
    pi_messages: Vec<crate::sessions::SessionMessage>,
) -> Vec<types::MessageResponse> {
    let mut all_messages: Vec<types::MessageResponse> = Vec::new();

    let mut prompt_iter = stored_prompts.into_iter().peekable();
    let mut pi_iter = pi_messages.into_iter().peekable();

    while prompt_iter.peek().is_some() || pi_iter.peek().is_some() {
        let take_prompt = match (prompt_iter.peek(), pi_iter.peek()) {
            (Some(prompt), Some(pi_msg)) => match (&prompt.timestamp, &pi_msg.timestamp) {
                (Some(p_ts), Some(m_ts)) => p_ts <= m_ts,
                (Some(_), None) => true,
                (None, Some(_)) => false,
                (None, None) => true,
            },
            (Some(_), None) => true,
            (None, Some(_)) => false,
            (None, None) => break,
        };

        if take_prompt {
            if let Some(prompt) = prompt_iter.next() {
                let response_images = prompt.images.map(|stored_imgs| {
                    stored_imgs
                        .into_iter()
                        .map(|img| types::ImageAttachmentResponse {
                            id: img.id,
                            filename: img.filename,
                            content_type: img.content_type,
                            size: img.size,
                            url: img.url,
                        })
                        .collect()
                });

                all_messages.push(types::MessageResponse {
                    role: prompt.role,
                    content: prompt.content,
                    timestamp: prompt.timestamp,
                    images: response_images,
                });
            }
        } else if let Some(pi_msg) = pi_iter.next() {
            all_messages.push(types::MessageResponse {
                role: pi_msg.role,
                content: pi_msg.content,
                timestamp: pi_msg.timestamp,
                images: None,
            });
        }
    }

    all_messages
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_id_from_path() {
        let path1 = PathBuf::from("/test/project");
        let path2 = PathBuf::from("/test/project");
        let path3 = PathBuf::from("/other/project");

        // Same path should generate same ID
        assert_eq!(project_id_from_path(&path1), project_id_from_path(&path2));
        // Different path should generate different ID
        assert_ne!(project_id_from_path(&path1), project_id_from_path(&path3));
    }

    #[test]
    fn test_project_name_from_path() {
        let path = PathBuf::from("/home/user/my-project");
        assert_eq!(project_name_from_path(&path), "my-project");
    }

    #[test]
    fn test_error_response_serialization() {
        let error = types::ErrorResponse {
            error: "NOT_FOUND".to_string(),
            message: "Test not found".to_string(),
        };

        let json = serde_json::to_string(&error).unwrap();
        assert!(json.contains("NOT_FOUND"));
        assert!(json.contains("Test not found"));
    }

    #[test]
    fn test_project_response_serialization() {
        let project = types::ProjectResponse {
            id: "test-id".to_string(),
            path: PathBuf::from("/test/project"),
            name: "test-project".to_string(),
            session_count: 5,
        };

        let json = serde_json::to_string(&project).unwrap();
        assert!(json.contains("test-id"));
        assert!(json.contains("test-project"));
        assert!(json.contains("5"));
    }

    #[test]
    fn test_session_response_serialization() {
        let session = types::SessionResponse {
            id: "session-123".to_string(),
            name: "Test Session".to_string(),
            project_id: "project-456".to_string(),
            project_path: PathBuf::from("/test/project"),
            created_at: "2025-01-13T00:00:00Z".to_string(),
            is_active: false,
        };

        let json = serde_json::to_string(&session).unwrap();
        assert!(json.contains("session-123"));
        assert!(json.contains("Test Session"));
        assert!(json.contains("project-456"));
    }
}