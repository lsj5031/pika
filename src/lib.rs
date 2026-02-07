mod api;
mod auth;
mod config;
mod file_watcher;
pub mod metrics;
mod pi;
mod sessions;
mod static_files;
mod websocket;

use axum::response::Json;
use serde_json::Value;

pub use api::{create_api_router, ApiState};
pub use auth::{AuthCredentials, basic_auth_middleware};
pub use config::ProjectConfig;
pub use file_watcher::{SessionFileEvent, SessionFileWatcher};
pub use pi::{ProcessManager, ProcessManagerEvent};
pub use sessions::{SessionIndex, build_session_index, build_encoded_project_map, load_session_info_from_file};
pub use static_files::serve_static_files;
pub use websocket::{WSEvent, WSState, ws_handler};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

/// Combined application state
#[derive(Clone)]
pub struct AppState {
    pub ws_state: WSState,
    pub api_state: ApiState,
    pub process_manager: Arc<Mutex<ProcessManager>>,
    pub session_index: Arc<RwLock<SessionIndex>>,
}

impl AppState {
    pub fn new(config: ProjectConfig) -> Self {
        Self {
            ws_state: WSState::new(),
            api_state: ApiState::new(config),
            process_manager: Arc::new(Mutex::new(ProcessManager::new())),
            session_index: Arc::new(RwLock::new(SessionIndex::empty())),
        }
    }
}

#[allow(dead_code)]
async fn health_check() -> Json<Value> {
    Json(serde_json::json!({
        "status": "ok",
        "service": "pika",
        "version": "0.1.0"
    }))
}

/// Test utilities - only available in test builds
#[cfg(any(test, feature = "test-utils"))]
pub mod test_utils {
    use super::*;
    use axum::{Router, routing::get};
    use axum::middleware;
    use tower_http::cors::{Any, CorsLayer};

    pub fn create_test_router() -> Router {
        let config = ProjectConfig::default();
        let app_state = AppState::new(config);
        let auth_credentials = AuthCredentials::new(String::new(), String::new());

        let protected_routes = Router::new()
            .merge(create_api_router())
            .with_state(app_state.clone())
            .layer(middleware::from_fn(move |req, next| {
                let creds = auth_credentials.clone();
                basic_auth_middleware(req, next, creds)
            }));

        Router::new()
            .route("/health", get(health_check))
            .route("/ws", get(ws_handler))
            .fallback(serve_static_files)
            .with_state(app_state)
            .merge(protected_routes)
            .layer(
                CorsLayer::new()
                    .allow_origin(Any)
                    .allow_methods(Any)
                    .allow_headers(Any),
            )
    }

    pub async fn create_test_app() -> Router {
        create_test_router()
    }
}

#[cfg(any(test, feature = "test-utils"))]
pub use test_utils::{create_test_app, create_test_router};
