mod api;
mod auth;
mod config;
mod file_watcher;
pub mod metrics;
mod agent;
mod rate_limit;
mod sessions;
mod static_files;
mod websocket;

pub use api::{ApiState, create_api_router, create_auth_router};
pub use auth::{
    AuthContext, AuthCredentials, SessionCookieManager, auth_middleware, is_request_authenticated,
};
pub use config::ProjectConfig;
pub use file_watcher::{SessionFileEvent, SessionFileWatcher};
use ipnet::IpNet;
pub use agent::{ProcessManager, ProcessManagerEvent};
pub use rate_limit::{RateLimitState, extract_client_ip};
pub use sessions::{
    SessionIndex, build_encoded_project_map, build_session_index, extract_message_content,
    load_session_info_from_file, pika_sessions_base_dir,
};
pub use static_files::serve_static_files;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;
use tokio::sync::{Mutex, RwLock};
pub use websocket::{WSEvent, WSState, ws_handler};

/// Combined application state
#[derive(Clone)]
pub struct AppState {
    pub ws_state: WSState,
    pub api_state: ApiState,
    pub process_manager: Arc<Mutex<ProcessManager>>,
    pub session_index: Arc<RwLock<SessionIndex>>,
    pub encoded_project_map: Arc<StdRwLock<HashMap<String, PathBuf>>>,
    pub auth_context: AuthContext,
    pub rate_limits: RateLimitState,
    pub trusted_proxy_cidrs: Arc<Vec<IpNet>>,
}

impl AppState {
    pub fn new(
        config: ProjectConfig,
        config_path: PathBuf,
        auth_context: AuthContext,
        rate_limits: RateLimitState,
    ) -> Self {
        let encoded_project_map = build_encoded_project_map(&config);
        let trusted_proxy_cidrs = Arc::new(rate_limit::parse_trusted_proxy_cidrs(
            &config.get_trusted_proxy_cidrs(),
        ));

        Self {
            ws_state: WSState::new(),
            api_state: ApiState::new(config, config_path),
            process_manager: Arc::new(Mutex::new(ProcessManager::new())),
            session_index: Arc::new(RwLock::new(SessionIndex::empty())),
            encoded_project_map: Arc::new(StdRwLock::new(encoded_project_map)),
            auth_context,
            rate_limits,
            trusted_proxy_cidrs,
        }
    }
}

pub async fn health_check() -> axum::response::Json<serde_json::Value> {
    axum::response::Json(serde_json::json!({
        "status": "ok",
        "service": "pika",
        "version": "0.1.0"
    }))
}

/// Test utilities - only available in test builds
#[cfg(any(test, feature = "test-utils"))]
pub mod test_utils {
    use super::*;
    use axum::extract::ConnectInfo;
    use axum::middleware::{self, Next};
    use axum::{Router, body::Body, http::Request, response::Response, routing::get};
    use std::net::SocketAddr;
    use tower_http::cors::{Any, CorsLayer};

    async fn inject_test_connect_info(mut req: Request<Body>, next: Next) -> Response {
        req.extensions_mut()
            .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 0))));
        next.run(req).await
    }

    pub fn create_test_router() -> Router {
        let config = ProjectConfig::default();
        // Auth disabled for most tests - login tests will verify empty credential handling
        let auth_context = AuthContext::new(
            AuthCredentials::new(String::new(), String::new()),
            false,
            SessionCookieManager::new(b"test-secret".to_vec(), 3600, false),
        );
        let app_state = AppState::new(
            config,
            PathBuf::from("config.toml"),
            auth_context.clone(),
            RateLimitState::new(100, 100),
        );

        let protected_routes = Router::new()
            .merge(create_api_router())
            .layer(middleware::from_fn(move |req, next| {
                let auth = auth_context.clone();
                auth_middleware(req, next, auth)
            }));

        Router::new()
            .route("/health", get(health_check))
            .route("/ws", get(ws_handler))
            .fallback(serve_static_files)
            .merge(create_auth_router())
            .merge(protected_routes)
            .with_state(app_state)
            .layer(middleware::from_fn(inject_test_connect_info))
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
