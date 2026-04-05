use axum::{
    extract::{Path, Query, State},
    response::Json,
};
use base64::Engine;
use std::collections::HashSet;
use tracing::{info, warn};

use crate::AppState;
use crate::agent::ImageUpload;
use crate::sessions::{
    CreateSessionRequest, create_session, get_session_messages_before,
    get_session_messages_limited, load_user_prompts,
};
use super::types::{
    CreateSessionInProjectRequest, CreateSessionInProjectResponse, CreateStandaloneSessionRequest,
    CreateStandaloneSessionResponse, ErrorResponse, MessageResponse,
    PagedResponse, PagedSessionsQuery, PromptRequest, SessionMessagesPagedQuery,
    SessionMessagesQuery, SessionResponse, SessionStatusResponse, SetThinkingLevelRequest,
    StartSessionResponse, StopSessionResponse,
};
use super::{
    DEFAULT_PAGE_LIMIT, MAX_PAGE_LIMIT, enforce_project_root_policy, find_session, merge_messages,
    resolve_canonical_path,
};

/// GET /api/sessions - list all sessions
pub async fn get_sessions(State(state): State<AppState>) -> Result<Json<Vec<SessionResponse>>, ErrorResponse> {
    let sessions = {
        let index = state.session_index.read().await;
        index.list_sorted(None, None)
    };

    let mut pm = state.process_manager.lock().await;
    let responses = sessions
        .into_iter()
        .map(|s| SessionResponse {
            id: s.id.clone(),
            name: s.name,
            project_id: crate::api::project_id_from_path(&s.project_path),
            project_path: s.project_path,
            created_at: s.created_at,
            is_active: pm.is_session_running(&s.id),
        })
        .collect();

    Ok(Json(responses))
}

/// GET /api/sessions/paged - list sessions with pagination
pub async fn get_sessions_paged(
    Query(query): Query<PagedSessionsQuery>,
    State(state): State<AppState>,
) -> Result<Json<PagedResponse<SessionResponse>>, ErrorResponse> {
    let limit = query
        .limit
        .unwrap_or(DEFAULT_PAGE_LIMIT)
        .min(MAX_PAGE_LIMIT);
    let cursor = query.cursor.as_deref();
    let q = query.q.as_deref();

    let page = {
        let index = state.session_index.read().await;
        index.paged(None, q, limit, cursor)
    };

    let mut pm = state.process_manager.lock().await;
    let data = page
        .sessions
        .into_iter()
        .map(|s| SessionResponse {
            id: s.id.clone(),
            name: s.name,
            project_id: crate::api::project_id_from_path(&s.project_path),
            project_path: s.project_path,
            created_at: s.created_at,
            is_active: pm.is_session_running(&s.id),
        })
        .collect();

    Ok(Json(PagedResponse {
        data,
        next_cursor: page.next_cursor,
        total: Some(page.total),
    }))
}

/// GET /api/projects/:id/sessions/paged - list sessions in a project with pagination
pub async fn get_project_sessions_paged(
    Path(id): Path<String>,
    Query(query): Query<PagedSessionsQuery>,
    State(state): State<AppState>,
) -> Result<Json<PagedResponse<SessionResponse>>, ErrorResponse> {
    let config = state.api_state.config.read().await;

    let project_path = config
        .project_root_paths
        .iter()
        .find(|path| crate::api::project_id_from_path(path) == id)
        .ok_or_else(|| ErrorResponse {
            error: "PROJECT_NOT_FOUND".to_string(),
            message: format!("Project with ID {} not found", id),
        })?;

    let limit = query
        .limit
        .unwrap_or(DEFAULT_PAGE_LIMIT)
        .min(MAX_PAGE_LIMIT);
    let cursor = query.cursor.as_deref();
    let q = query.q.as_deref();

    let page = {
        let index = state.session_index.read().await;
        index.paged(Some(project_path), q, limit, cursor)
    };

    let mut pm = state.process_manager.lock().await;
    let data = page
        .sessions
        .into_iter()
        .map(|s| SessionResponse {
            id: s.id.clone(),
            name: s.name,
            project_id: id.clone(),
            project_path: s.project_path,
            created_at: s.created_at,
            is_active: pm.is_session_running(&s.id),
        })
        .collect();

    Ok(Json(PagedResponse {
        data,
        next_cursor: page.next_cursor,
        total: Some(page.total),
    }))
}

/// POST /api/sessions/lookup - look up multiple sessions by ID
pub async fn lookup_sessions(
    State(state): State<AppState>,
    Json(payload): Json<super::types::SessionsLookupRequest>,
) -> Result<Json<Vec<SessionResponse>>, ErrorResponse> {
    let sessions = {
        let index = state.session_index.read().await;
        index.lookup(&payload.ids)
    };

    let mut pm = state.process_manager.lock().await;
    let responses = sessions
        .into_iter()
        .map(|s| SessionResponse {
            id: s.id.clone(),
            name: s.name,
            project_id: crate::api::project_id_from_path(&s.project_path),
            project_path: s.project_path,
            created_at: s.created_at,
            is_active: pm.is_session_running(&s.id),
        })
        .collect();

    Ok(Json(responses))
}

/// GET /api/sessions/:id - get session details
pub async fn get_session(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<SessionResponse>, ErrorResponse> {
    let session = find_session(&state, &id).await.ok_or_else(|| ErrorResponse {
        error: "NOT_FOUND".to_string(),
        message: format!("Session {} not found", id),
    })?;

    let mut pm = state.process_manager.lock().await;
    Ok(Json(SessionResponse {
        id: session.id,
        name: session.name,
        project_id: crate::api::project_id_from_path(&session.project_path),
        project_path: session.project_path,
        created_at: session.created_at,
        is_active: pm.is_session_running(&id),
    }))
}

/// GET /api/sessions/:id/messages - get messages for a session
pub async fn get_session_messages(
    Path(id): Path<String>,
    Query(query): Query<SessionMessagesQuery>,
    State(state): State<AppState>,
) -> Result<Json<Vec<MessageResponse>>, ErrorResponse> {
    let session = find_session(&state, &id).await.ok_or_else(|| ErrorResponse {
        error: "NOT_FOUND".to_string(),
        message: format!("Session {} not found", id),
    })?;

    let limit = query.limit;
    let from_start = query.direction.as_deref() == Some("head");

    // Load stored user prompts (sent via API) and pi-agent persisted messages
    let (stored_prompts, pi_messages) = tokio::join!(
        load_user_prompts(&id, &session.project_path),
        tokio::task::spawn_blocking({
            let id = id.clone();
            let path = session.project_path.clone();
            move || get_session_messages_limited(&id, &path, limit, from_start)
        })
    );

    let pi_messages = pi_messages.unwrap().map_err(|e| ErrorResponse {
        error: "INTERNAL_ERROR".to_string(),
        message: format!("Failed to load messages: {}", e),
    })?;

    // Interleave messages based on timestamp
    let all_messages = merge_messages(stored_prompts, pi_messages);

    // Apply limit to the merged list if requested
    let final_messages = if let Some(l) = limit {
        if from_start {
            all_messages.into_iter().take(l).collect()
        } else {
            let start = all_messages.len().saturating_sub(l);
            all_messages.into_iter().skip(start).collect()
        }
    } else {
        all_messages
    };

    Ok(Json(final_messages))
}

/// GET /api/sessions/:id/messages/paged - get messages for a session with pagination
pub async fn get_session_messages_paged(
    Path(id): Path<String>,
    Query(query): Query<SessionMessagesPagedQuery>,
    State(state): State<AppState>,
) -> Result<Json<Vec<MessageResponse>>, ErrorResponse> {
    let session = find_session(&state, &id).await.ok_or_else(|| ErrorResponse {
        error: "NOT_FOUND".to_string(),
        message: format!("Session {} not found", id),
    })?;

    let limit = query.limit.unwrap_or(DEFAULT_PAGE_LIMIT).min(MAX_PAGE_LIMIT);
    let before = query.before.as_deref();

    // For paged loading, we only look at the session.jsonl file for now
    // (User prompts stored separately aren't easily paged without loading all)
    let messages = tokio::task::spawn_blocking({
        let id = id.clone();
        let path = session.project_path.clone();
        let before = before.map(|s| s.to_string());
        move || get_session_messages_before(&id, &path, limit, before.as_deref())
    })
    .await
    .unwrap()
    .map_err(|e| ErrorResponse {
        error: "INTERNAL_ERROR".to_string(),
        message: format!("Failed to load messages: {}", e),
    })?;

    let responses = messages
        .into_iter()
        .map(|m| MessageResponse {
            role: m.role,
            content: m.content,
            timestamp: m.timestamp,
            images: None,
        })
        .collect();

    Ok(Json(responses))
}

/// POST /api/sessions/:id/prompt - send a prompt to an active session
pub async fn send_prompt_to_session(
    Path(session_id): Path<String>,
    State(state): State<AppState>,
    Json(req): Json<PromptRequest>,
) -> Result<Json<serde_json::Value>, ErrorResponse> {
    let session = find_session(&state, &session_id)
        .await
        .ok_or_else(|| ErrorResponse {
            error: "NOT_FOUND".to_string(),
            message: format!("Session {} not found", session_id),
        })?;

    // Validate prompt length
    let max_chars = {
        let config = state.api_state.config.read().await;
        config.max_prompt_chars
    };
    if req.prompt.len() > max_chars {
        return Err(ErrorResponse {
            error: "BAD_REQUEST".to_string(),
            message: format!("Prompt exceeds maximum length of {} characters", max_chars),
        });
    }

    // Process image attachments
    let mut image_uploads = Vec::new();
    let mut total_image_size = 0;

    let config = state.api_state.config.read().await;
    let allowed_mimes: HashSet<String> = config.allowed_image_mime_types.iter().cloned().collect();

    if let Some(images) = &req.images {
        if images.len() > config.max_images_per_prompt {
            return Err(ErrorResponse {
                error: "BAD_REQUEST".to_string(),
                message: format!(
                    "Too many images (max {} allowed)",
                    config.max_images_per_prompt
                ),
            });
        }

        for img in images {
            if img.content_type.is_empty()
                || img.content_type == "undefined"
                || !allowed_mimes.contains(&img.content_type)
            {
                return Err(ErrorResponse {
                    error: "BAD_REQUEST".to_string(),
                    message: format!("Invalid or unsupported image type: {}", img.content_type),
                });
            }

            if img.data.is_empty() || img.data == "undefined" {
                return Err(ErrorResponse {
                    error: "BAD_REQUEST".to_string(),
                    message: format!("Invalid image data for file: {}", img.filename),
                });
            }

            // Simple base64 size estimation
            let estimated_size = (img.data.len() * 3) / 4;
            if estimated_size > config.max_image_bytes {
                return Err(ErrorResponse {
                    error: "PAYLOAD_TOO_LARGE".to_string(),
                    message: format!(
                        "Image {} exceeds maximum size of {} bytes",
                        img.filename, config.max_image_bytes
                    ),
                });
            }

            total_image_size += estimated_size;
            image_uploads.push(ImageUpload {
                filename: img.filename.clone(),
                content_type: img.content_type.clone(),
                data: img.data.clone(),
            });
        }
    }

    if total_image_size > config.max_total_image_bytes {
        return Err(ErrorResponse {
            error: "PAYLOAD_TOO_LARGE".to_string(),
            message: format!(
                "Total image size exceeds maximum of {} bytes",
                config.max_total_image_bytes
            ),
        });
    }

    drop(config);

    let mut pm = state.process_manager.lock().await;

    // Check if session has an active process
    let process_id = if let Some(pid) = pm.get_process_id_for_session(&session_id) {
        if pm.is_running(&pid) {
            pid
        } else {
            // Spawn new process if it exited
            pm.spawn_for_session(&session_id, session.project_path.clone())
                .map_err(|e| ErrorResponse {
                    error: "INTERNAL_ERROR".to_string(),
                    message: format!("Failed to spawn pika-agent: {}", e),
                })?
        }
    } else {
        // Spawn new process
        pm.spawn_for_session(&session_id, session.project_path.clone())
            .map_err(|e| ErrorResponse {
                error: "INTERNAL_ERROR".to_string(),
                message: format!("Failed to spawn pika-agent: {}", e),
            })?
    };

    // Send prompt to process
    pm.send_prompt_with_images(&process_id, &req.prompt, &image_uploads)
        .await
        .map_err(|e| ErrorResponse {
            error: "INTERNAL_ERROR".to_string(),
            message: format!("Failed to send prompt: {}", e),
        })?;

    // Store the user prompt for later retrieval (since pi-agent doesn't store user prompts in rpc mode)
    let decoded_image_sizes: Vec<usize> = image_uploads
        .iter()
        .map(|img| base64::engine::general_purpose::STANDARD.decode(&img.data).map(|d| d.len()).unwrap_or(0))
        .collect();

    let images_to_store: Vec<crate::sessions::ImageAttachmentStored> = req.images
        .unwrap_or_default()
        .into_iter()
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
            Some(images_to_store.clone())
        },
    )
    .await
    {
        warn!(error = %e, session_id = %session_id, "Failed to store user prompt");
    }

    // Broadcast user prompt via WebSocket so it appears immediately in the chat
    let ws_images = if images_to_store.is_empty() {
        None
    } else {
        Some(
            images_to_store
                .into_iter()
                .map(|img| crate::api::types::ImageAttachmentResponse {
                    id: img.id,
                    filename: img.filename,
                    content_type: img.content_type,
                    size: img.size,
                    url: img.url,
                })
                .collect(),
        )
    };

    let ws_event = crate::websocket::WSEvent::MessageAdded {
        session_id: session_id.clone(),
        role: "user".to_string(),
        content: req.prompt.clone(),
        timestamp: chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        images: ws_images,
    };
    state.ws_state.broadcast(ws_event);

    Ok(Json(serde_json::json!({
        "status": "ok",
        "session_id": session_id,
        "process_id": process_id,
        "message": "Prompt sent successfully"
    })))
}

/// POST /api/sessions/:id/start - start/resume a session
pub async fn start_session(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<StartSessionResponse>, ErrorResponse> {
    let session = find_session(&state, &id).await.ok_or_else(|| ErrorResponse {
        error: "NOT_FOUND".to_string(),
        message: format!("Session {} not found", id),
    })?;

    let mut pm = state.process_manager.lock().await;

    // Check if already running
    if let Some(process_id) = pm.get_process_id_for_session(&id) {
        if pm.is_running(&process_id) {
            return Ok(Json(StartSessionResponse {
                process_id,
                newly_spawned: false,
            }));
        }
    }

    // Spawn new process
    let process_id = pm
        .spawn_for_session(&id, session.project_path)
        .map_err(|e| ErrorResponse {
            error: "INTERNAL_ERROR".to_string(),
            message: format!("Failed to start session: {}", e),
        })?;

    Ok(Json(StartSessionResponse {
        process_id,
        newly_spawned: true,
    }))
}

/// GET /api/sessions/:id/status - get session process status
pub async fn get_session_status(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<SessionStatusResponse>, ErrorResponse> {
    let mut pm = state.process_manager.lock().await;

    let process_id = pm.get_process_id_for_session(&id);
    let is_running = process_id.as_ref().map(|pid| pm.is_running(pid)).unwrap_or(false);

    Ok(Json(SessionStatusResponse {
        session_id: id,
        is_running,
        process_id: if is_running { process_id } else { None },
    }))
}

/// POST /api/sessions/:id/stop - stop a session process
pub async fn stop_session(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<StopSessionResponse>, ErrorResponse> {
    let mut pm = state.process_manager.lock().await;

    let process_id = pm.get_process_id_for_session(&id);
    let is_running = process_id.as_ref().map(|pid| pm.is_running(pid)).unwrap_or(false);

    if let Some(pid) = process_id {
        if is_running {
            pm.kill(&pid).await.map_err(|e| ErrorResponse {
                error: "INTERNAL_ERROR".to_string(),
                message: format!("Failed to stop session: {}", e),
            })?;

            return Ok(Json(StopSessionResponse {
                session_id: id,
                process_id: Some(pid),
                was_running: true,
            }));
        }
    }

    Ok(Json(StopSessionResponse {
        session_id: id,
        process_id: None,
        was_running: false,
    }))
}

/// POST /api/sessions/:id/cycle-thinking-level - cycle thinking level for a session
pub async fn cycle_thinking_level(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, ErrorResponse> {
    let mut pm = state.process_manager.lock().await;

    let process_id = pm.get_process_id_for_session(&id);
    let is_running = process_id.as_ref().map(|pid| pm.is_running(pid)).unwrap_or(false);

    if !is_running {
        return Err(ErrorResponse {
            error: "NOT_RUNNING".to_string(),
            message: "Session process is not running".to_string(),
        });
    }

    let pid = process_id.unwrap();
    pm.send_command(&pid, serde_json::json!({ "type": "cycle_thinking_level" }))
        .await
        .map_err(|e| ErrorResponse {
            error: "INTERNAL_ERROR".to_string(),
            message: format!("Failed to cycle thinking level: {}", e),
        })?;

    Ok(Json(serde_json::json!({ "success": true })))
}

/// POST /api/sessions/:id/set-thinking-level - set thinking level for a session
pub async fn set_thinking_level(
    Path(id): Path<String>,
    State(state): State<AppState>,
    Json(payload): Json<SetThinkingLevelRequest>,
) -> Result<Json<serde_json::Value>, ErrorResponse> {
    let mut pm = state.process_manager.lock().await;

    let process_id = pm.get_process_id_for_session(&id);
    let is_running = process_id.as_ref().map(|pid| pm.is_running(pid)).unwrap_or(false);

    if !is_running {
        return Err(ErrorResponse {
            error: "NOT_RUNNING".to_string(),
            message: "Session process is not running".to_string(),
        });
    }

    let pid = process_id.unwrap();
    pm.send_command(
        &pid,
        serde_json::json!({ "type": "set_thinking_level", "level": payload.level }),
    )
    .await
    .map_err(|e| ErrorResponse {
        error: "INTERNAL_ERROR".to_string(),
        message: format!("Failed to set thinking level: {}", e),
    })?;

    Ok(Json(serde_json::json!({ "success": true })))
}

/// POST /api/sessions/create - create a new standalone session
pub async fn create_standalone_session(
    State(state): State<AppState>,
    Json(payload): Json<CreateStandaloneSessionRequest>,
) -> Result<Json<CreateStandaloneSessionResponse>, ErrorResponse> {
    let absolute_path = resolve_canonical_path(&payload.path)?;

    // Enforce root path policy
    enforce_project_root_policy(&state, &absolute_path).await?;

    // Create session
    let response = tokio::task::spawn_blocking({
        let path = absolute_path.clone();
        let name = payload.name.clone();
        move || create_session(&path, CreateSessionRequest { name })
    })
    .await
    .unwrap()
    .map_err(|e| ErrorResponse {
        error: "INTERNAL_ERROR".to_string(),
        message: format!("Failed to create session: {}", e),
    })?;

    // Rebuild session index to include new session
    let config = state.api_state.config.read().await;
    let new_index = crate::sessions::build_session_index(&config).await;
    {
        let mut index = state.session_index.write().await;
        *index = new_index;
    }

    info!(path = %absolute_path.display(), id = %response.session_id, "Standalone session created");

    Ok(Json(CreateStandaloneSessionResponse {
        session_id: response.session_id,
        name: response.name,
        path: absolute_path,
        created_at: response.created_at,
    }))
}

/// POST /api/projects/:id/sessions - create a new session in a project
pub async fn create_session_in_project(
    Path(id): Path<String>,
    State(state): State<AppState>,
    Json(payload): Json<CreateSessionInProjectRequest>,
) -> Result<Json<CreateSessionInProjectResponse>, ErrorResponse> {
    let config = state.api_state.config.read().await;

    let project_path = config
        .project_root_paths
        .iter()
        .find(|path| crate::api::project_id_from_path(path) == id)
        .ok_or_else(|| ErrorResponse {
            error: "PROJECT_NOT_FOUND".to_string(),
            message: format!("Project with ID {} not found", id),
        })?
        .clone();

    drop(config);

    // Create session
    let session_response = tokio::task::spawn_blocking({
        let path = project_path.clone();
        let name = payload.name.clone();
        move || create_session(&path, CreateSessionRequest { name })
    })
    .await
    .unwrap()
    .map_err(|e| ErrorResponse {
        error: "INTERNAL_ERROR".to_string(),
        message: format!("Failed to create session: {}", e),
    })?;

    // Start process for the new session
    let mut pm = state.process_manager.lock().await;
    let process_id = pm
        .spawn_for_session(&session_response.session_id, project_path)
        .map_err(|e| ErrorResponse {
            error: "INTERNAL_ERROR".to_string(),
            message: format!("Failed to start session: {}", e),
        })?;

    // Rebuild session index
    let config = state.api_state.config.read().await;
    let new_index = crate::sessions::build_session_index(&config).await;
    {
        let mut index = state.session_index.write().await;
        *index = new_index;
    }

    info!(id = %session_response.session_id, "Session created in project {}", id);

    Ok(Json(CreateSessionInProjectResponse {
        session_id: session_response.session_id,
        name: session_response.name,
        project_id: id,
        project_path: session_response.project_path,
        created_at: session_response.created_at,
        newly_spawned: true,
        process_id: Some(process_id),
    }))
}
