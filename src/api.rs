use axum::{
    Router,
    extract::{Path, State, Query},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::{delete, get, post},
};
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::AppState;
use crate::config::ProjectConfig;
use crate::pi::ImageUpload;
use crate::sessions::{
    CreateSessionRequest, SessionMessage, build_session_index, create_session,
    get_session_messages_before, get_session_messages_limited, load_user_prompts,
};
use std::sync::Arc;
use tokio::sync::RwLock;

/// API state shared across all handlers
#[derive(Clone)]
pub struct ApiState {
    pub config: Arc<RwLock<ProjectConfig>>,
}

impl ApiState {
    pub fn new(config: ProjectConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
        }
    }
}

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
            "NOT_FOUND" => StatusCode::NOT_FOUND,
            "BAD_REQUEST" => StatusCode::BAD_REQUEST,
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

const DEFAULT_PAGE_LIMIT: usize = 50;
const MAX_PAGE_LIMIT: usize = 200;

async fn find_session(state: &AppState, session_id: &str) -> Option<crate::sessions::SessionInfo> {
    let index = state.session_index.read().await;
    index.get(session_id).cloned()
}

fn merge_messages(
    stored_prompts: Vec<SessionMessage>,
    pi_messages: Vec<SessionMessage>,
) -> Vec<MessageResponse> {
    let mut all_messages: Vec<MessageResponse> = Vec::new();

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
                        .map(|img| ImageAttachmentResponse {
                            id: img.id,
                            filename: img.filename,
                            content_type: img.content_type,
                            size: img.size,
                            url: img.url,
                        })
                        .collect()
                });

                all_messages.push(MessageResponse {
                    role: prompt.role,
                    content: prompt.content,
                    timestamp: prompt.timestamp,
                    images: response_images,
                });
            }
        } else if let Some(pi_msg) = pi_iter.next() {
            all_messages.push(MessageResponse {
                role: pi_msg.role,
                content: pi_msg.content,
                timestamp: pi_msg.timestamp,
                images: None,
            });
        }
    }

    all_messages
}

/// Generate a project ID from path (simple hash-based approach)
fn project_id_from_path(path: &PathBuf) -> String {
    // Use a simple hash of the path string as ID
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

/// Extract project name from path
fn project_name_from_path(path: &std::path::Path) -> String {
    path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown")
        .to_string()
}

/// GET /api/projects - returns list of configured projects
pub async fn get_projects(
    State(state): State<AppState>,
) -> Result<Json<Vec<ProjectResponse>>, ErrorResponse> {
    let config = state.api_state.config.read().await;
    let index = state.session_index.read().await;
    let session_counts = index.project_counts();

    // Build project responses
    let projects: Vec<ProjectResponse> = config
        .project_root_paths
        .iter()
        .map(|path| {
            let project_id = project_id_from_path(path);
            ProjectResponse {
                id: project_id.clone(),
                path: path.clone(),
                name: project_name_from_path(path),
                session_count: session_counts.get(path).copied().unwrap_or(0),
            }
        })
        .collect();

    Ok(Json(projects))
}

/// GET /api/auth/status - returns whether auth is enabled
pub async fn get_auth_status(
    State(state): State<AppState>,
) -> Result<Json<AuthStatusResponse>, ErrorResponse> {
    let config = state.api_state.config.read().await;
    Ok(Json(AuthStatusResponse {
        enabled: config.is_auth_enabled(),
    }))
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

/// POST /api/projects - add a new project to config
pub async fn add_project(
    State(state): State<AppState>,
    Json(request): Json<AddProjectRequest>,
) -> Result<Json<AddProjectResponse>, ErrorResponse> {
    // Expand ~ to home directory
    let expanded_path = if request.path.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            home.join(&request.path[2..])
        } else {
            PathBuf::from(&request.path)
        }
    } else {
        PathBuf::from(&request.path)
    };

    // Convert to absolute path
    let absolute_path = if expanded_path.is_absolute() {
        expanded_path
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(&expanded_path)
    };

    // Validate path exists
    if !absolute_path.exists() {
        return Err(ErrorResponse {
            error: "INVALID_PATH".to_string(),
            message: format!("Path does not exist: {}", absolute_path.display()),
        });
    }

    let project_id = project_id_from_path(&absolute_path);

    // Update config
    {
        let mut config = state.api_state.config.write().await;

        // Check if project already exists
        if config.project_root_paths.contains(&absolute_path) {
            return Err(ErrorResponse {
                error: "PROJECT_EXISTS".to_string(),
                message: format!("Project already exists: {}", absolute_path.display()),
            });
        }

        // Add project
        config.project_root_paths.push(absolute_path.clone());

        // Save to config file
        if let Err(e) = config.to_file("config.toml") {
            return Err(ErrorResponse {
                error: "CONFIG_SAVE_FAILED".to_string(),
                message: format!("Failed to save config: {}", e),
            });
        }
    }

    let config = state.api_state.config.read().await;
    let rebuilt = build_session_index(&config).await;
    let mut index = state.session_index.write().await;
    *index = rebuilt;

    Ok(Json(AddProjectResponse {
        id: project_id,
        name: project_name_from_path(&absolute_path),
        path: absolute_path,
    }))
}

/// DELETE /api/projects/:id - remove a project from config
pub async fn remove_project(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> Result<Json<serde_json::Value>, ErrorResponse> {
    // Update config
    {
        let mut config = state.api_state.config.write().await;

        // Find and remove the project
        let original_len = config.project_root_paths.len();
        config
            .project_root_paths
            .retain(|path| project_id_from_path(path) != project_id);

        if config.project_root_paths.len() == original_len {
            return Err(ErrorResponse {
                error: "PROJECT_NOT_FOUND".to_string(),
                message: format!("Project not found: {}", project_id),
            });
        }

        // Save to config file
        if let Err(e) = config.to_file("config.toml") {
            return Err(ErrorResponse {
                error: "CONFIG_SAVE_FAILED".to_string(),
                message: format!("Failed to save config: {}", e),
            });
        }
    }

    let config = state.api_state.config.read().await;
    let rebuilt = build_session_index(&config).await;
    let mut index = state.session_index.write().await;
    *index = rebuilt;

    Ok(Json(serde_json::json!({ "success": true })))
}

/// GET /api/projects/:id/sessions - returns sessions for a specific project
pub async fn get_project_sessions(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> Result<Json<Vec<SessionResponse>>, ErrorResponse> {
    let config = state.api_state.config.read().await;

    // Find the project path by ID
    let project_path = config
        .project_root_paths
        .iter()
        .find(|path| project_id_from_path(path) == project_id);

    let project_path = match project_path {
        Some(path) => path,
        None => {
            return Err(ErrorResponse {
                error: "NOT_FOUND".to_string(),
                message: format!("Project with ID '{}' not found", project_id),
            });
        }
    };

    let index = state.session_index.read().await;
    let project_sessions = index
        .list_sorted(Some(project_path), None)
        .into_iter()
        .map(|s| SessionResponse {
            id: s.id,
            name: s.name,
            project_id: project_id.clone(),
            project_path: s.project_path,
            created_at: s.created_at,
            is_active: s.is_active,
        })
        .collect();

    Ok(Json(project_sessions))
}

/// GET /api/projects/:id/sessions/paged - returns sessions for a project with pagination
pub async fn get_project_sessions_paged(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Query(query): Query<PagedSessionsQuery>,
) -> Result<Json<PagedResponse<SessionResponse>>, ErrorResponse> {
    let config = state.api_state.config.read().await;

    let project_path = config
        .project_root_paths
        .iter()
        .find(|path| project_id_from_path(path) == project_id);

    let project_path = match project_path {
        Some(path) => path,
        None => {
            return Err(ErrorResponse {
                error: "NOT_FOUND".to_string(),
                message: format!("Project with ID '{}' not found", project_id),
            });
        }
    };

    let limit = query
        .limit
        .unwrap_or(DEFAULT_PAGE_LIMIT)
        .min(MAX_PAGE_LIMIT);

    let index = state.session_index.read().await;
    let page = index.paged(
        Some(project_path),
        query.q.as_deref(),
        limit,
        query.cursor.as_deref(),
    );

    let data = page
        .sessions
        .into_iter()
        .map(|s| SessionResponse {
            id: s.id,
            name: s.name,
            project_id: project_id.clone(),
            project_path: s.project_path,
            created_at: s.created_at,
            is_active: s.is_active,
        })
        .collect();

    Ok(Json(PagedResponse {
        data,
        next_cursor: page.next_cursor,
        total: Some(page.total),
    }))
}

/// GET /api/sessions - returns all sessions across all projects
pub async fn get_sessions(
    State(state): State<AppState>,
) -> Result<Json<Vec<SessionResponse>>, ErrorResponse> {
    let index = state.session_index.read().await;
    let session_responses: Vec<SessionResponse> = index
        .list_sorted(None, None)
        .into_iter()
        .map(|s| {
            let project_id = project_id_from_path(&s.project_path);
            SessionResponse {
                id: s.id,
                name: s.name,
                project_id,
                project_path: s.project_path,
                created_at: s.created_at,
                is_active: s.is_active,
            }
        })
        .collect();

    Ok(Json(session_responses))
}

/// GET /api/sessions/paged - returns sessions with pagination
pub async fn get_sessions_paged(
    State(state): State<AppState>,
    Query(query): Query<PagedSessionsQuery>,
) -> Result<Json<PagedResponse<SessionResponse>>, ErrorResponse> {
    let limit = query
        .limit
        .unwrap_or(DEFAULT_PAGE_LIMIT)
        .min(MAX_PAGE_LIMIT);
    let index = state.session_index.read().await;
    let page = index.paged(None, query.q.as_deref(), limit, query.cursor.as_deref());

    let data = page
        .sessions
        .into_iter()
        .map(|s| SessionResponse {
            id: s.id,
            name: s.name,
            project_id: project_id_from_path(&s.project_path),
            project_path: s.project_path,
            created_at: s.created_at,
            is_active: s.is_active,
        })
        .collect();

    Ok(Json(PagedResponse {
        data,
        next_cursor: page.next_cursor,
        total: Some(page.total),
    }))
}

/// POST /api/sessions/lookup - fetch sessions by IDs
pub async fn lookup_sessions(
    State(state): State<AppState>,
    Json(request): Json<SessionsLookupRequest>,
) -> Result<Json<Vec<SessionResponse>>, ErrorResponse> {
    if request.ids.is_empty() {
        return Ok(Json(Vec::new()));
    }

    let index = state.session_index.read().await;
    let sessions = index.lookup(&request.ids);

    let response = sessions
        .into_iter()
        .map(|s| SessionResponse {
            id: s.id,
            name: s.name,
            project_id: project_id_from_path(&s.project_path),
            project_path: s.project_path,
            created_at: s.created_at,
            is_active: s.is_active,
        })
        .collect();

    Ok(Json(response))
}

/// GET /api/sessions/:id - returns session details and metadata
pub async fn get_session(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Result<Json<SessionResponse>, ErrorResponse> {
    let session = match find_session(&state, &session_id).await {
        Some(s) => s,
        None => {
            return Err(ErrorResponse {
                error: "NOT_FOUND".to_string(),
                message: format!("Session with ID '{}' not found", session_id),
            });
        }
    };

    let project_id = project_id_from_path(&session.project_path);

    Ok(Json(SessionResponse {
        id: session.id,
        name: session.name,
        project_id,
        project_path: session.project_path,
        created_at: session.created_at,
        is_active: session.is_active,
    }))
}

/// GET /api/sessions/:id/messages - returns messages for a session
pub async fn get_session_messages(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Query(query): Query<SessionMessagesQuery>,
) -> Result<Json<Vec<MessageResponse>>, ErrorResponse> {
    let session = match find_session(&state, &session_id).await {
        Some(s) => s,
        None => {
            return Err(ErrorResponse {
                error: "NOT_FOUND".to_string(),
                message: format!("Session with ID '{}' not found", session_id),
            });
        }
    };

    // Load stored user prompts that we sent via our API
    let stored_prompts = load_user_prompts(&session.id, &session.project_path);

    let requested_limit = query.limit;
    if let Some(0) = requested_limit {
        return Ok(Json(Vec::new()));
    }

    let direction = query.direction.as_deref().unwrap_or("tail");
    let from_start = direction == "head";

    let pi_messages = if requested_limit.is_some() {
        get_session_messages_limited(
            &session.id,
            &session.project_path,
            requested_limit,
            from_start,
        )
    } else {
        get_session_messages_limited(&session.id, &session.project_path, None, false)
    }
    .map_err(|e| ErrorResponse {
        error: "INTERNAL_ERROR".to_string(),
        message: format!("Failed to read messages: {}", e),
    })?;

    Ok(Json(merge_messages(stored_prompts, pi_messages)))
}

/// GET /api/sessions/:id/messages/paged - returns paged messages for a session
pub async fn get_session_messages_paged(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Query(query): Query<SessionMessagesPagedQuery>,
) -> Result<Json<Vec<MessageResponse>>, ErrorResponse> {
    let session = match find_session(&state, &session_id).await {
        Some(s) => s,
        None => {
            return Err(ErrorResponse {
                error: "NOT_FOUND".to_string(),
                message: format!("Session with ID '{}' not found", session_id),
            });
        }
    };

    let limit = query
        .limit
        .unwrap_or(DEFAULT_PAGE_LIMIT)
        .min(MAX_PAGE_LIMIT);

    let stored_prompts = load_user_prompts(&session.id, &session.project_path)
        .into_iter()
        .filter(|prompt| match query.before.as_deref() {
            Some(before) => prompt.timestamp.as_deref().map(|ts| ts < before).unwrap_or(false),
            None => true,
        })
        .collect::<Vec<_>>();

    let pi_messages = get_session_messages_before(
        &session.id,
        &session.project_path,
        limit,
        query.before.as_deref(),
    )
    .map_err(|e| ErrorResponse {
        error: "INTERNAL_ERROR".to_string(),
        message: format!("Failed to read messages: {}", e),
    })?;

    let mut merged = merge_messages(stored_prompts, pi_messages);
    if merged.len() > limit {
        merged = merged[merged.len() - limit..].to_vec();
    }

    Ok(Json(merged))
}

/// POST /api/sessions/:id/prompt - send a prompt to a session
pub async fn send_prompt_to_session(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Json(req): Json<PromptRequest>,
) -> Result<Json<serde_json::Value>, ErrorResponse> {
    let session = match find_session(&state, &session_id).await {
        Some(s) => s,
        None => {
            return Err(ErrorResponse {
                error: "NOT_FOUND".to_string(),
                message: format!("Session with ID '{}' not found", session_id),
            });
        }
    };

    // Convert images to the format expected by pi-coding-agent
    let images_to_send: Vec<ImageUpload> = req
        .images
        .as_ref()
        .map_or(&Vec::new(), |v| v)
        .iter()
        .map(|img| ImageUpload {
            filename: img.filename.clone(),
            content_type: img.content_type.clone(),
            data: img.data.clone(),
        })
        .collect();

    // Lock the process manager
    let mut process_manager = state.process_manager.lock().await;

    // Check if the session is already running
    let process_id = if process_manager.is_session_running(&session_id) {
        process_manager
            .get_process_id_for_session(&session_id)
            .unwrap()
    } else {
        process_manager
            .spawn_for_session(&session_id, session.project_path.clone())
            .map_err(|e| ErrorResponse {
                error: "INTERNAL_ERROR".to_string(),
                message: format!("Failed to start session: {}", e),
            })?
    };

    // Send the prompt with images to the process
    process_manager
        .send_prompt_with_images(&process_id, &req.prompt, &images_to_send)
        .await
        .map_err(|e| ErrorResponse {
            error: "INTERNAL_ERROR".to_string(),
            message: format!("Failed to send prompt: {}", e),
        })?;

    // Store the user prompt with image metadata for later retrieval
    let images_to_store: Vec<crate::sessions::ImageAttachmentStored> = req
        .images
        .as_ref()
        .map_or(&Vec::new(), |v| v)
        .iter()
        .map(|img| {
            let actual_size = base64::engine::general_purpose::STANDARD
                .decode(&img.data)
                .map(|decoded| decoded.len())
                .unwrap_or(img.data.len());

            crate::sessions::ImageAttachmentStored {
                id: uuid::Uuid::new_v4().to_string(),
                filename: img.filename.clone(),
                content_type: img.content_type.clone(),
                size: actual_size,
                url: format!("data:{};base64,{}", img.content_type, img.data),
            }
        })
        .collect();

    if let Err(e) = crate::sessions::store_user_prompt_with_images(
        &session_id,
        &session.project_path,
        &req.prompt,
        if images_to_store.is_empty() {
            None
        } else {
            Some(images_to_store)
        },
    ) {
        eprintln!("Failed to store user prompt: {}", e);
    }

    Ok(Json(serde_json::json!({
        "status": "ok",
        "session_id": session_id,
        "process_id": process_id,
        "message": "Prompt sent successfully"
    })))
}

/// POST /api/sessions/:id/start - start a session's process (or return if already running)
pub async fn start_session(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Result<Json<StartSessionResponse>, ErrorResponse> {
    let session = match find_session(&state, &session_id).await {
        Some(s) => s,
        None => {
            return Err(ErrorResponse {
                error: "NOT_FOUND".to_string(),
                message: format!("Session with ID '{}' not found", session_id),
            });
        }
    };

    // Lock the process manager and start the session
    let mut process_manager = state.process_manager.lock().await;

    // Check if the session is already running
    let already_running = process_manager.is_session_running(&session_id);

    // Start the session (will return existing process ID if already running)
    let process_id = process_manager
        .spawn_for_session(&session_id, session.project_path.clone())
        .map_err(|e| ErrorResponse {
            error: "INTERNAL_ERROR".to_string(),
            message: format!("Failed to start session: {}", e),
        })?;

    Ok(Json(StartSessionResponse {
        process_id,
        newly_spawned: !already_running,
    }))
}

/// GET /api/sessions/:id/status - check if a session is currently running
pub async fn get_session_status(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Result<Json<SessionStatusResponse>, ErrorResponse> {
    if find_session(&state, &session_id).await.is_none() {
        return Err(ErrorResponse {
            error: "NOT_FOUND".to_string(),
            message: format!("Session with ID '{}' not found", session_id),
        });
    }

    // Lock the process manager and check if the session is running
    let mut process_manager = state.process_manager.lock().await;
    let is_running = process_manager.is_session_running(&session_id);

    // Get the process ID if running
    let process_id = if is_running {
        process_manager.get_process_id_for_session(&session_id)
    } else {
        None
    };

    Ok(Json(SessionStatusResponse {
        session_id,
        is_running,
        process_id,
    }))
}

/// POST /api/sessions/:id/stop - stop a running session's process
pub async fn stop_session(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Result<Json<StopSessionResponse>, ErrorResponse> {
    if find_session(&state, &session_id).await.is_none() {
        return Err(ErrorResponse {
            error: "NOT_FOUND".to_string(),
            message: format!("Session with ID '{}' not found", session_id),
        });
    }

    // Lock the process manager and get the process ID before killing
    let mut process_manager = state.process_manager.lock().await;
    let is_running = process_manager.is_session_running(&session_id);
    let process_id = if is_running {
        process_manager.get_process_id_for_session(&session_id)
    } else {
        None
    };

    // Release the lock before killing (kill needs mutable access)
    drop(process_manager);

    // Kill the process if it was running
    let mut process_manager = state.process_manager.lock().await;

    if let Some(pid) = &process_id
        && is_running
    {
        process_manager.kill(pid).await.map_err(|e| ErrorResponse {
            error: "INTERNAL_ERROR".to_string(),
            message: format!("Failed to stop session: {}", e),
        })?;
    }

    Ok(Json(StopSessionResponse {
        session_id,
        process_id,
        was_running: is_running,
    }))
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

/// POST /api/sessions/create - create a new session in any folder
pub async fn create_standalone_session(
    State(state): State<AppState>,
    Json(request): Json<CreateStandaloneSessionRequest>,
) -> Result<Json<CreateStandaloneSessionResponse>, ErrorResponse> {
    use crate::sessions::{CreateSessionRequest, create_session};

    // Expand ~ to home directory
    let expanded_path = if request.path.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            home.join(&request.path[2..])
        } else {
            PathBuf::from(&request.path)
        }
    } else {
        PathBuf::from(&request.path)
    };

    // Convert to absolute path and canonicalize
    let absolute_path = if expanded_path.is_absolute() {
        expanded_path
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(&expanded_path)
    };

    // Canonicalize the path
    let canonical_path = absolute_path.canonicalize().map_err(|_| ErrorResponse {
        error: "INVALID_PATH".to_string(),
        message: format!("Cannot canonicalize path: {}", absolute_path.display()),
    })?;

    // Validate path exists
    if !canonical_path.exists() {
        return Err(ErrorResponse {
            error: "INVALID_PATH".to_string(),
            message: format!("Path does not exist: {}", canonical_path.display()),
        });
    }

    // Create the session
    let create_request = CreateSessionRequest { name: request.name };

    let result = create_session(&canonical_path, create_request).map_err(|e| ErrorResponse {
        error: "SESSION_CREATE_FAILED".to_string(),
        message: format!("Failed to create session: {}", e),
    })?;

    // Register the folder path in project_root_paths for discoverability
    // This ensures the session shows up in the session list
    {
        let mut config = state.api_state.config.write().await;
        // Only add if not already present
        if !config.project_root_paths.contains(&canonical_path) {
            config.project_root_paths.push(canonical_path.clone());
        }
    }

    Ok(Json(CreateStandaloneSessionResponse {
        session_id: result.session_id,
        name: result.name,
        path: canonical_path,
        created_at: result.created_at,
    }))
}

/// POST /api/projects/:id/sessions - create a new session in a project
pub async fn create_session_in_project(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Json(req): Json<CreateSessionInProjectRequest>,
) -> Result<Json<CreateSessionInProjectResponse>, ErrorResponse> {
    // Get config to find project path by ID
    let config = state.api_state.config.read().await;

    // Find the project path by ID
    let project_path = config
        .project_root_paths
        .iter()
        .find(|path| project_id_from_path(path) == project_id)
        .cloned(); // Clone the PathBuf to avoid borrow issues

    let project_path = match project_path {
        Some(path) => path,
        None => {
            return Err(ErrorResponse {
                error: "NOT_FOUND".to_string(),
                message: format!("Project with ID '{}' not found", project_id),
            });
        }
    };

    // Drop config read lock before doing file I/O
    drop(config);

    // Create the session
    let create_request = CreateSessionRequest { name: req.name };

    let session_response =
        create_session(&project_path, create_request).map_err(|e| ErrorResponse {
            error: "INTERNAL_ERROR".to_string(),
            message: format!("Failed to create session: {}", e),
        })?;

    let session_id = session_response.session_id.clone();

    // Spawn the pi process immediately in RPC mode
    let mut process_manager = state.process_manager.lock().await;

    let process_id = process_manager
        .spawn_for_session(&session_id, project_path.clone())
        .map_err(|e| ErrorResponse {
            error: "INTERNAL_ERROR".to_string(),
            message: format!("Failed to spawn session process: {}", e),
        })?;

    Ok(Json(CreateSessionInProjectResponse {
        session_id: session_response.session_id,
        name: session_response.name,
        project_id,
        project_path: session_response.project_path,
        created_at: session_response.created_at,
        newly_spawned: true,
        process_id: Some(process_id),
    }))
}

/// Create the API router with all endpoints
pub fn create_api_router() -> Router<AppState> {
    Router::new()
        .route("/api/auth/status", get(get_auth_status))
        .route("/api/projects", get(get_projects).post(add_project))
        .route("/api/projects/{id}", delete(remove_project))
        .route("/api/projects/{id}/sessions", get(get_project_sessions))
        .route(
            "/api/projects/{id}/sessions/paged",
            get(get_project_sessions_paged),
        )
        .route(
            "/api/projects/{id}/sessions",
            post(create_session_in_project),
        )
        .route("/api/sessions", get(get_sessions))
        .route("/api/sessions/paged", get(get_sessions_paged))
        .route("/api/sessions/lookup", post(lookup_sessions))
        .route("/api/sessions/create", post(create_standalone_session))
        .route("/api/sessions/{id}", get(get_session))
        .route("/api/sessions/{id}/messages", get(get_session_messages))
        .route(
            "/api/sessions/{id}/messages/paged",
            get(get_session_messages_paged),
        )
        .route("/api/sessions/{id}/prompt", post(send_prompt_to_session))
        .route("/api/sessions/{id}/status", get(get_session_status))
        .route("/api/sessions/{id}/start", post(start_session))
        .route("/api/sessions/{id}/stop", post(stop_session))
        .route(
            "/api/settings",
            get(get_pi_settings).post(update_pi_settings),
        )
}

/// PI settings response
#[derive(Debug, Serialize)]
pub struct PiSettingsResponse {
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
pub struct UpdatePiSettingsRequest {
    /// Default model
    #[serde(rename = "defaultModel", skip_serializing_if = "Option::is_none")]
    pub default_model: Option<String>,
    /// Default thinking level
    #[serde(rename = "defaultThinkingLevel", skip_serializing_if = "Option::is_none")]
    pub default_thinking_level: Option<String>,
    /// Default provider
    #[serde(rename = "defaultProvider", skip_serializing_if = "Option::is_none")]
    pub default_provider: Option<String>,
    /// Hide thinking block
    #[serde(rename = "hideThinkingBlock", skip_serializing_if = "Option::is_none")]
    pub hide_thinking_block: Option<bool>,
}

/// GET /api/settings - get PI settings
pub async fn get_pi_settings(
    State(_state): State<AppState>,
) -> Result<Json<PiSettingsResponse>, ErrorResponse> {
    use std::fs;
    use std::path::PathBuf;

    let pi_agent_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".pi")
        .join("agent");

    let settings_path = pi_agent_dir.join("settings.json");
    let models_path = pi_agent_dir.join("models.json");

    // Read settings
    let settings = if settings_path.exists() {
        fs::read_to_string(&settings_path)
            .ok()
            .and_then(|content| serde_json::from_str(&content).ok())
            .unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    // Read models
    let models = if models_path.exists() {
        fs::read_to_string(&models_path)
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

    let response = PiSettingsResponse {
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
    State(_state): State<AppState>,
    Json(request): Json<UpdatePiSettingsRequest>,
) -> Result<Json<serde_json::Value>, ErrorResponse> {
    use std::fs;
    use std::path::PathBuf;

    let pi_agent_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".pi")
        .join("agent");

    let settings_path = pi_agent_dir.join("settings.json");

    // Read existing settings
    let mut settings = if settings_path.exists() {
        fs::read_to_string(&settings_path)
            .ok()
            .and_then(|content| serde_json::from_str(&content).ok())
            .unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

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
        fs::create_dir_all(parent).map_err(|e| ErrorResponse {
            error: "INTERNAL_ERROR".to_string(),
            message: format!("Failed to create settings directory: {}", e),
        })?;
    }

    // Write settings
    fs::write(
        &settings_path,
        serde_json::to_string_pretty(&settings).unwrap(),
    )
    .map_err(|e| ErrorResponse {
        error: "INTERNAL_ERROR".to_string(),
        message: format!("Failed to write settings: {}", e),
    })?;

    Ok(Json(serde_json::json!({ "success": true })))
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
        let error = ErrorResponse {
            error: "NOT_FOUND".to_string(),
            message: "Test not found".to_string(),
        };

        let json = serde_json::to_string(&error).unwrap();
        assert!(json.contains("NOT_FOUND"));
        assert!(json.contains("Test not found"));
    }

    #[test]
    fn test_project_response_serialization() {
        let project = ProjectResponse {
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
        let session = SessionResponse {
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
