use axum::{
    extract::{Path, State},
    response::Json,
};
use tracing::{info, warn};

use crate::AppState;
use super::types::{
    AddProjectRequest, AddProjectResponse, ErrorResponse, ProjectResponse, SessionResponse,
};
use super::{enforce_project_root_policy, resolve_canonical_path};

/// GET /api/projects - list all projects
pub async fn get_projects(State(state): State<AppState>) -> Result<Json<Vec<ProjectResponse>>, ErrorResponse> {
    let config = state.api_state.config.read().await;
    let counts = {
        let index = state.session_index.read().await;
        index.project_counts()
    };

    let projects = config
        .project_root_paths
        .iter()
        .map(|path| {
            let id = crate::api::project_id_from_path(path);
            let name = crate::api::project_name_from_path(path);
            let session_count = counts.get(path).copied().unwrap_or(0);

            ProjectResponse {
                id,
                path: path.clone(),
                name,
                session_count,
            }
        })
        .collect();

    Ok(Json(projects))
}

/// POST /api/projects - add a new project
pub async fn add_project(
    State(state): State<AppState>,
    Json(payload): Json<AddProjectRequest>,
) -> Result<Json<AddProjectResponse>, ErrorResponse> {
    let absolute_path = resolve_canonical_path(&payload.path)?;

    // Enforce root path policy
    enforce_project_root_policy(&state, &absolute_path).await?;

    let mut config = state.api_state.config.write().await;

    // Check if project already exists
    if config.project_root_paths.contains(&absolute_path) {
        return Err(ErrorResponse {
            error: "PROJECT_EXISTS".to_string(),
            message: format!("Project already exists: {}", absolute_path.display()),
        });
    }

    config.project_root_paths.push(absolute_path.clone());

    // Save configuration
    if let Err(e) = config.to_file(&state.api_state.config_path) {
        warn!(error = %e, "Failed to save configuration after adding project");
    }

    // Rebuild session index
    let new_index = crate::sessions::build_session_index(&config).await;
    {
        let mut index = state.session_index.write().await;
        *index = new_index;
    }

    // Update encoded project map
    let new_map = crate::sessions::build_encoded_project_map(&config);
    {
        let mut map = state.encoded_project_map.write().unwrap();
        *map = new_map;
    }

    info!(path = %absolute_path.display(), "Project added");

    Ok(Json(AddProjectResponse {
        id: crate::api::project_id_from_path(&absolute_path),
        name: crate::api::project_name_from_path(&absolute_path),
        path: absolute_path,
    }))
}

/// DELETE /api/projects/:id - remove a project
pub async fn remove_project(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, ErrorResponse> {
    let mut config = state.api_state.config.write().await;

    let index = config
        .project_root_paths
        .iter()
        .position(|path| crate::api::project_id_from_path(path) == id)
        .ok_or_else(|| ErrorResponse {
            error: "PROJECT_NOT_FOUND".to_string(),
            message: format!("Project with ID {} not found", id),
        })?;

    let removed_path = config.project_root_paths.remove(index);

    // Save configuration
    if let Err(e) = config.to_file(&state.api_state.config_path) {
        warn!(error = %e, "Failed to save configuration after removing project");
    }

    // Rebuild session index
    let new_index = crate::sessions::build_session_index(&config).await;
    {
        let mut index = state.session_index.write().await;
        *index = new_index;
    }

    // Update encoded project map
    let new_map = crate::sessions::build_encoded_project_map(&config);
    {
        let mut map = state.encoded_project_map.write().unwrap();
        *map = new_map;
    }

    info!(path = %removed_path.display(), "Project removed");

    Ok(Json(serde_json::json!({ "success": true })))
}

/// GET /api/projects/:id/sessions - list sessions in a project
pub async fn get_project_sessions(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<Vec<SessionResponse>>, ErrorResponse> {
    let config = state.api_state.config.read().await;

    let project_path = config
        .project_root_paths
        .iter()
        .find(|path| crate::api::project_id_from_path(path) == id)
        .ok_or_else(|| ErrorResponse {
            error: "PROJECT_NOT_FOUND".to_string(),
            message: format!("Project with ID {} not found", id),
        })?;

    let sessions = {
        let index = state.session_index.read().await;
        index.list_sorted(Some(project_path), None)
    };

    let mut pm = state.process_manager.lock().await;
    let responses = sessions
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

    Ok(Json(responses))
}
