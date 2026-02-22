use axum::{
    Router,
    routing::{delete, get, post},
};

use crate::AppState;

use super::settings::{get_pi_settings, update_pi_settings};
use super::{
    add_project, create_session_in_project, create_standalone_session, cycle_thinking_level,
    get_auth_status, get_project_sessions, get_project_sessions_paged, get_projects, get_session,
    get_session_messages, get_session_messages_paged, get_session_status, get_sessions,
    get_sessions_paged, login, logout, lookup_sessions, remove_project, send_prompt_to_session,
    set_thinking_level, start_session, stop_session,
};

/// Public auth router (unprotected endpoints)
pub fn create_auth_router() -> Router<AppState> {
    Router::new()
        .route("/api/auth/status", get(get_auth_status))
        .route("/api/auth/login", post(login))
        .route("/api/auth/logout", post(logout))
}

/// Create the protected API router with all endpoints
pub fn create_api_router() -> Router<AppState> {
    Router::new()
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
            "/api/sessions/{id}/cycle-thinking-level",
            post(cycle_thinking_level),
        )
        .route(
            "/api/sessions/{id}/set-thinking-level",
            post(set_thinking_level),
        )
        .route(
            "/api/settings",
            get(get_pi_settings).post(update_pi_settings),
        )
}
