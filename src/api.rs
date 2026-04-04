mod routes;
mod settings;
mod types;

pub use routes::{create_api_router, create_auth_router};
pub use types::*;

use axum::{
    extract::{ConnectInfo, Path, Query, State},
    http::{HeaderMap, header},
    response::{IntoResponse, Json, Response},
};
use base64::Engine;
use std::collections::HashSet;
use std::net::SocketAddr;
use std::path::PathBuf;

use crate::auth::is_request_authenticated;
use crate::config::ProjectConfig;
use crate::agent::ImageUpload;
use crate::sessions::{
    CreateSessionRequest, SessionMessage, build_encoded_project_map, build_session_index,
    create_session, get_session_messages_before, get_session_messages_limited, load_user_prompts,
};
use crate::{AppState, extract_client_ip};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::warn;

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

const DEFAULT_PAGE_LIMIT: usize = 50;
const MAX_PAGE_LIMIT: usize = 200;

fn resolve_canonical_path(input_path: &str) -> Result<PathBuf, ErrorResponse> {
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

    absolute_path.canonicalize().map_err(|_| ErrorResponse {
        error: "INVALID_PATH".to_string(),
        message: format!("Cannot canonicalize path: {}", absolute_path.display()),
    })
}

async fn enforce_project_root_policy(
    state: &AppState,
    candidate_path: &std::path::Path,
) -> Result<(), ErrorResponse> {
    let config = state.api_state.config.read().await;

    if config.is_path_allowed(candidate_path) {
        Ok(())
    } else {
        Err(ErrorResponse {
            error: "INVALID_PATH".to_string(),
            message: format!(
                "Path '{}' is outside configured allowed project roots",
                candidate_path.display()
            ),
        })
    }
}

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

/// GET /api/auth/status - returns auth mode and whether request is already authenticated
pub async fn get_auth_status(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AuthStatusResponse>, ErrorResponse> {
    let config = state.api_state.config.read().await;
    let enabled = config.is_auth_enabled();
    drop(config);

    let authenticated = if enabled {
        is_request_authenticated(&headers, &state.auth_context)
    } else {
        true
    };

    Ok(Json(AuthStatusResponse {
        enabled,
        authenticated,
    }))
}

/// POST /api/auth/login - validate credentials and issue session cookie
pub async fn login(
    State(state): State<AppState>,
    ConnectInfo(peer_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(request): Json<LoginRequest>,
) -> Result<Response, ErrorResponse> {
    if !state.auth_context.auth_enabled {
        return Ok(Json(LoginResponse {
            success: true,
            expires_in_seconds: state.auth_context.session_cookie.ttl_seconds(),
        })
        .into_response());
    }

    let client_ip = extract_client_ip(
        &headers,
        peer_addr,
        state.trusted_proxy_cidrs.as_ref().as_slice(),
    );
    let decision = state.rate_limits.login.check(&client_ip.to_string()).await;
    if !decision.allowed {
        return Err(ErrorResponse {
            error: "TOO_MANY_REQUESTS".to_string(),
            message: format!(
                "Too many login attempts. Try again in {}s.",
                decision.retry_after_seconds
            ),
        });
    }

    if !state
        .auth_context
        .credentials
        .validate(&request.username, &request.password)
    {
        return Err(ErrorResponse {
            error: "UNAUTHORIZED".to_string(),
            message: "Invalid username or password".to_string(),
        });
    }

    let cookie = state
        .auth_context
        .session_cookie
        .issue_session_cookie(&request.username);

    Ok((
        [(header::SET_COOKIE, cookie)],
        Json(LoginResponse {
            success: true,
            expires_in_seconds: state.auth_context.session_cookie.ttl_seconds(),
        }),
    )
        .into_response())
}

/// POST /api/auth/logout - clear session cookie
pub async fn logout(State(state): State<AppState>) -> Result<Response, ErrorResponse> {
    let clear_cookie = state.auth_context.session_cookie.clear_session_cookie();

    Ok((
        [(header::SET_COOKIE, clear_cookie)],
        Json(serde_json::json!({ "success": true })),
    )
        .into_response())
}

/// POST /api/projects - add a new project to config
pub async fn add_project(
    State(state): State<AppState>,
    Json(request): Json<AddProjectRequest>,
) -> Result<Json<AddProjectResponse>, ErrorResponse> {
    let canonical_path = resolve_canonical_path(&request.path)?;

    // Validate path exists
    if !canonical_path.exists() {
        return Err(ErrorResponse {
            error: "INVALID_PATH".to_string(),
            message: format!("Path does not exist: {}", canonical_path.display()),
        });
    }

    enforce_project_root_policy(&state, &canonical_path).await?;

    let project_id = project_id_from_path(&canonical_path);

    // Update config
    let config_path = state.api_state.config_path.clone();
    let updated_encoded_map = {
        let mut config = state.api_state.config.write().await;

        // Check if project already exists
        if config.project_root_paths.contains(&canonical_path) {
            return Err(ErrorResponse {
                error: "PROJECT_EXISTS".to_string(),
                message: format!("Project already exists: {}", canonical_path.display()),
            });
        }

        // Add project
        config.project_root_paths.push(canonical_path.clone());

        // Save to config file
        if let Err(e) = config.to_file(&config_path) {
            return Err(ErrorResponse {
                error: "CONFIG_SAVE_FAILED".to_string(),
                message: format!("Failed to save config: {}", e),
            });
        }
        build_encoded_project_map(&config)
    };

    if let Ok(mut map) = state.encoded_project_map.write() {
        *map = updated_encoded_map;
    } else {
        warn!("Encoded project map lock poisoned after add_project");
    }

    let config = state.api_state.config.read().await;
    let rebuilt = build_session_index(&config).await;
    let mut index = state.session_index.write().await;
    *index = rebuilt;

    Ok(Json(AddProjectResponse {
        id: project_id,
        name: project_name_from_path(&canonical_path),
        path: canonical_path,
    }))
}

/// DELETE /api/projects/:id - remove a project from config
pub async fn remove_project(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> Result<Json<serde_json::Value>, ErrorResponse> {
    // Update config
    let config_path = state.api_state.config_path.clone();
    let updated_encoded_map = {
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
        if let Err(e) = config.to_file(&config_path) {
            return Err(ErrorResponse {
                error: "CONFIG_SAVE_FAILED".to_string(),
                message: format!("Failed to save config: {}", e),
            });
        }
        build_encoded_project_map(&config)
    };

    if let Ok(mut map) = state.encoded_project_map.write() {
        *map = updated_encoded_map;
    } else {
        warn!("Encoded project map lock poisoned after remove_project");
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
    let stored_prompts = load_user_prompts(&session.id, &session.project_path).await;

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
        .await
        .into_iter()
        .filter(|prompt| match query.before.as_deref() {
            Some(before) => prompt
                .timestamp
                .as_deref()
                .map(|ts| ts < before)
                .unwrap_or(false),
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

    if req.prompt.trim().is_empty() {
        return Err(ErrorResponse {
            error: "VALIDATION_ERROR".to_string(),
            message: "Prompt must not be empty".to_string(),
        });
    }

    let config = state.api_state.config.read().await;
    if req.prompt.chars().count() > config.max_prompt_chars {
        return Err(ErrorResponse {
            error: "PAYLOAD_TOO_LARGE".to_string(),
            message: format!(
                "Prompt exceeds maximum length of {} characters",
                config.max_prompt_chars
            ),
        });
    }

    let request_images: &[ImageAttachment] = req.images.as_deref().unwrap_or(&[]);
    if request_images.len() > config.max_images_per_prompt {
        return Err(ErrorResponse {
            error: "VALIDATION_ERROR".to_string(),
            message: format!(
                "Too many images: {} (max {})",
                request_images.len(),
                config.max_images_per_prompt
            ),
        });
    }

    let allowed_mime_types: HashSet<String> = config
        .allowed_image_mime_types
        .iter()
        .map(|mime| mime.to_lowercase())
        .collect();

    let max_image_bytes = config.max_image_bytes;
    let max_total_image_bytes = config.max_total_image_bytes;
    drop(config);

    let mut decoded_image_sizes = Vec::with_capacity(request_images.len());
    let mut total_decoded_bytes = 0usize;

    for image in request_images {
        let content_type = image.content_type.to_lowercase();
        if !allowed_mime_types.contains(&content_type) {
            return Err(ErrorResponse {
                error: "VALIDATION_ERROR".to_string(),
                message: format!("Unsupported image MIME type: {}", image.content_type),
            });
        }

        let decoded = base64::engine::general_purpose::STANDARD
            .decode(&image.data)
            .map_err(|_| ErrorResponse {
                error: "VALIDATION_ERROR".to_string(),
                message: format!("Image '{}' has invalid base64 payload", image.filename),
            })?;

        let decoded_len = decoded.len();
        if decoded_len > max_image_bytes {
            return Err(ErrorResponse {
                error: "PAYLOAD_TOO_LARGE".to_string(),
                message: format!(
                    "Image '{}' exceeds per-image size limit of {} bytes",
                    image.filename, max_image_bytes
                ),
            });
        }

        total_decoded_bytes = total_decoded_bytes.saturating_add(decoded_len);
        if total_decoded_bytes > max_total_image_bytes {
            return Err(ErrorResponse {
                error: "PAYLOAD_TOO_LARGE".to_string(),
                message: format!(
                    "Total image payload exceeds limit of {} bytes",
                    max_total_image_bytes
                ),
            });
        }

        decoded_image_sizes.push(decoded_len);
    }

    // Convert images to the format expected by pika-agent
    let images_to_send: Vec<ImageUpload> = request_images
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
        .map_err(|e| match &e {
            crate::agent::PikaProcessError::ProcessNotRunning { .. } => ErrorResponse {
                error: "SESSION_STOPPED".to_string(),
                message: "Session process has stopped. Please restart the session.".to_string(),
            },
            _ => ErrorResponse {
                error: "INTERNAL_ERROR".to_string(),
                message: format!("Failed to send prompt: {}", e),
            },
        })?;

    // Store the user prompt with image metadata for later retrieval
    let images_to_store: Vec<crate::sessions::ImageAttachmentStored> = request_images
        .iter()
        .enumerate()
        .map(|(index, img)| crate::sessions::ImageAttachmentStored {
            id: uuid::Uuid::new_v4().to_string(),
            filename: img.filename.clone(),
            content_type: img.content_type.clone(),
            size: decoded_image_sizes.get(index).copied().unwrap_or(0),
            url: format!("data:{};base64,{}", img.content_type, img.data),
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
    )
    .await
    {
        warn!(error = %e, session_id = %session_id, "Failed to store user prompt");
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

/// POST /api/sessions/:id/cycle-thinking-level - cycle thinking level on the running process
pub async fn cycle_thinking_level(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Result<Json<serde_json::Value>, ErrorResponse> {
    let mut process_manager = state.process_manager.lock().await;

    let process_id = process_manager
        .get_process_id_for_session(&session_id)
        .ok_or_else(|| ErrorResponse {
            error: "NOT_RUNNING".to_string(),
            message: format!("Session '{}' has no running process", session_id),
        })?;

    process_manager
        .send_command(
            &process_id,
            serde_json::json!({ "type": "cycle_thinking_level" }),
        )
        .await
        .map_err(|e| ErrorResponse {
            error: "INTERNAL_ERROR".to_string(),
            message: format!("Failed to cycle thinking level: {}", e),
        })?;

    Ok(Json(serde_json::json!({
        "status": "ok",
        "session_id": session_id,
    })))
}

/// POST /api/sessions/:id/set-thinking-level - set thinking level on the running process
pub async fn set_thinking_level(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Json(req): Json<SetThinkingLevelRequest>,
) -> Result<Json<serde_json::Value>, ErrorResponse> {
    let mut process_manager = state.process_manager.lock().await;

    let process_id = process_manager
        .get_process_id_for_session(&session_id)
        .ok_or_else(|| ErrorResponse {
            error: "NOT_RUNNING".to_string(),
            message: format!("Session '{}' has no running process", session_id),
        })?;

    process_manager
        .send_command(
            &process_id,
            serde_json::json!({
                "type": "set_thinking_level",
                "level": req.level,
            }),
        )
        .await
        .map_err(|e| ErrorResponse {
            error: "INTERNAL_ERROR".to_string(),
            message: format!("Failed to set thinking level: {}", e),
        })?;

    Ok(Json(serde_json::json!({
        "status": "ok",
        "session_id": session_id,
        "level": req.level,
    })))
}

/// POST /api/sessions/create - create a new session in any folder
pub async fn create_standalone_session(
    State(state): State<AppState>,
    Json(request): Json<CreateStandaloneSessionRequest>,
) -> Result<Json<CreateStandaloneSessionResponse>, ErrorResponse> {
    use crate::sessions::{CreateSessionRequest, create_session};

    let canonical_path = resolve_canonical_path(&request.path)?;

    // Validate path exists
    if !canonical_path.exists() {
        return Err(ErrorResponse {
            error: "INVALID_PATH".to_string(),
            message: format!("Path does not exist: {}", canonical_path.display()),
        });
    }

    enforce_project_root_policy(&state, &canonical_path).await?;

    // Create the session
    let create_request = CreateSessionRequest { name: request.name };

    let result = create_session(&canonical_path, create_request).map_err(|e| ErrorResponse {
        error: "SESSION_CREATE_FAILED".to_string(),
        message: format!("Failed to create session: {}", e),
    })?;

    // Register the folder path in project_root_paths for discoverability
    // This ensures the session shows up in the session list
    let config_path = state.api_state.config_path.clone();
    let updated_encoded_map = {
        let mut config = state.api_state.config.write().await;
        // Only add if not already present
        if !config.project_root_paths.contains(&canonical_path) {
            config.project_root_paths.push(canonical_path.clone());

            if let Err(e) = config.to_file(&config_path) {
                return Err(ErrorResponse {
                    error: "CONFIG_SAVE_FAILED".to_string(),
                    message: format!("Failed to save config: {}", e),
                });
            }
        }
        build_encoded_project_map(&config)
    };

    if let Ok(mut map) = state.encoded_project_map.write() {
        *map = updated_encoded_map;
    } else {
        warn!("Encoded project map lock poisoned after create_standalone_session");
    }

    let config = state.api_state.config.read().await;
    let rebuilt = build_session_index(&config).await;
    let mut index = state.session_index.write().await;
    *index = rebuilt;

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

    // Spawn the Pika process immediately in RPC mode
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
