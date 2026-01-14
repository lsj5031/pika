use axum::{
    middleware,
    routing::get,
    Router,
    response::Json,
};
use clap::Parser;
use serde_json::Value;
use std::net::SocketAddr;
use std::path::PathBuf;
use tower_http::cors::{CorsLayer, Any};
use tokio::net::TcpListener;

mod auth;
mod config;
mod sessions;
mod websocket;
mod pi;
mod api;
mod static_files;
use auth::AuthCredentials;
use config::ProjectConfig;
use websocket::{WSState, WSEvent};
use api::ApiState;
use pi::ProcessManager;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Combined application state
#[derive(Clone)]
pub struct AppState {
    pub ws_state: WSState,
    pub api_state: ApiState,
    pub process_manager: Arc<Mutex<ProcessManager>>,
}

impl AppState {
    pub fn new(config: ProjectConfig) -> Self {
        Self {
            ws_state: WSState::new(),
            api_state: ApiState::new(config),
            process_manager: Arc::new(Mutex::new(ProcessManager::new())),
        }
    }
}

/// Pi Agent Manager - Manages multiple agent sessions and their execution contexts
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Path to the configuration file (default: ./config.toml)
    #[arg(short, long, global = true)]
    config: Option<PathBuf>,
    
    /// Port to listen on (default: 3000, overrides PORT env var)
    #[arg(short, long)]
    port: Option<u16>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse CLI arguments
    let cli = Cli::parse();

    // Determine config file path (default: ./config.toml)
    let config_path = cli.config.unwrap_or_else(|| PathBuf::from("config.toml"));
    
    // Load configuration
    println!("📄 Loading configuration from: {}", config_path.display());
    let config = ProjectConfig::from_file(&config_path)?;
    
    // Validate configuration
    config.validate()?;
    
    println!("✅ Configuration loaded successfully");
    if config.project_root_paths.is_empty() {
        println!("⚠️  No project root paths configured");
    } else {
        println!("📁 Monitoring {} project root path(s):", config.project_root_paths.len());
        for path in &config.project_root_paths {
            println!("   - {}", path.display());
        }
    }
    
    // Load port from CLI arg, environment variable, or use default
    let port = cli.port
        .or_else(|| std::env::var("PORT").ok().and_then(|p| p.parse().ok()))
        .unwrap_or(7847);
    
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    
    // Create combined application state
    let app_state = AppState::new(config.clone());

    // Set up auth credentials
    let auth_credentials = AuthCredentials::new(
        config.get_auth_username().unwrap_or_default(),
        config.get_auth_password().unwrap_or_default(),
    );
    let auth_enabled = auth_credentials.is_enabled();

    // Build protected routes (require auth via HTTP header if enabled)
    let protected_routes = Router::new()
        .merge(api::create_api_router())
        .fallback(static_files::serve_static_files)
        .with_state(app_state.clone())
        .layer(middleware::from_fn(move |req, next| {
            let creds = auth_credentials.clone();
            auth::basic_auth_middleware(req, next, creds)
        }));

    // Build the full application
    // - /health is always public
    // - /ws handles its own auth via query params (WebSocket doesn't support headers)
    // - All other routes use HTTP Basic Auth middleware
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/ws", get(websocket::ws_handler))
        .with_state(app_state.clone())
        .merge(protected_routes)
        .layer(
            // CORS layer for local development and production
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        );

    println!("🚀 Pi Agent Manager server listening on http://{}", addr);
    println!("📡 WebSocket endpoint available at ws://{}", addr);
    if auth_enabled {
        println!("🔐 HTTP Basic Auth enabled");
    } else {
        println!("⚠️  HTTP Basic Auth disabled (no credentials configured)");
    }

    let listener = TcpListener::bind(addr).await?;

    // Start event bridging task
    let app_state_for_bridge = app_state.clone();
    tokio::spawn(async move {
        event_bridge_task(app_state_for_bridge).await;
    });

    axum::serve(listener, app).await?;

    Ok(())
}

/// Background task that bridges ProcessManager events to WebSocket events
async fn event_bridge_task(app_state: AppState) {
    use pi::ProcessManagerEvent;

    let mut rx = {
        let pm = app_state.process_manager.lock().await;
        pm.subscribe()
    };

    println!("📡 Event bridge task started");

    while let Ok(event) = rx.recv().await {
        match event {
            ProcessManagerEvent::ProcessSpawned { id, project_path } => {
                println!("🚀 Process spawned: {} in {}", id, project_path.display());

                // Look up the session ID for this process
                let session_id = {
                    let pm = app_state.process_manager.lock().await;
                    pm.get_session_id_for_process(&id)
                };

                // Use session_id if found, otherwise fall back to process_id
                let ws_id = session_id.unwrap_or_else(|| id.clone());

                let ws_event = WSEvent::SessionStarted {
                    session_id: ws_id,
                    project_path: project_path.to_string_lossy().to_string(),
                };
                app_state.ws_state.broadcast(ws_event);
            }
            ProcessManagerEvent::ProcessKilled { id } => {
                println!("🛑 Process killed: {}", id);

                // Look up the session ID for this process
                let session_id = {
                    let pm = app_state.process_manager.lock().await;
                    pm.get_session_id_for_process(&id)
                };

                // Use session_id if found, otherwise fall back to process_id
                let ws_id = session_id.unwrap_or_else(|| id.clone());

                let ws_event = WSEvent::SessionStopped {
                    session_id: ws_id,
                };
                app_state.ws_state.broadcast(ws_event);
            }
            ProcessManagerEvent::SessionStarted { session_id, process_id } => {
                println!("🎯 Session started: {} (process: {})", session_id, process_id);
                // SessionStarted is already handled by ProcessSpawned, so this is redundant
                // but we keep it for future use if we need to distinguish between the two
            }
            ProcessManagerEvent::JsonRpc { id, event } => {
                // Look up the session ID for this process
                let session_id = {
                    let pm = app_state.process_manager.lock().await;
                    pm.get_session_id_for_process(&id)
                };

                // Use session_id if found, otherwise fall back to process_id
                let ws_id = session_id.unwrap_or_else(|| id.clone());

                // Convert pi event to WSEvent
                // pi-coding-agent sends events with "type" field
                if let Some(event_type) = &event.event_type {
                    match event_type.as_str() {
                        "message_update" => {
                            // Streaming update - check if it's thinking or text
                            if let Some(msg_event) = event.extra.get("assistantMessageEvent") {
                                if let Some(delta_type) = msg_event.get("type").and_then(|t| t.as_str()) {
                                    match delta_type {
                                        "thinking_delta" => {
                                            let content = msg_event.get("delta")
                                                .and_then(|v| v.as_str())
                                                .unwrap_or("");

                                            if !content.is_empty() {
                                                let ws_event = WSEvent::ThinkingDelta {
                                                    session_id: ws_id.clone(),
                                                    content: content.to_string(),
                                                };
                                                app_state.ws_state.broadcast(ws_event);
                                            }
                                        }
                                        "text_delta" => {
                                            // Text streaming - could broadcast this too if we want real-time text
                                            // For now, we'll wait for message_end to add the complete message
                                        }
                                        _ => {
                                            // Other delta types (toolcall, etc.)
                                        }
                                    }
                                }
                            }
                        }
                        "message_end" => {
                            // Message completed - extract role and content
                            if let Some(message) = event.extra.get("message") {
                                let role = message.get("role")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("assistant");

                                // Get timestamp
                                let timestamp = message.get("timestamp")
                                    .and_then(|v| v.as_i64())
                                    .map(|ts| {
                                        // Convert milliseconds to seconds if needed
                                        // Millisecond timestamps are > 10_000_000_000_000 (year 2286)
                                        let ts_seconds = if ts > 10_000_000_000_000 {
                                            ts / 1000
                                        } else {
                                            ts
                                        };

                                        // Use from_timestamp for conversion (returns Option, handles both secs and ms)
                                        chrono::DateTime::from_timestamp(ts_seconds, 0)
                                            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                                            .unwrap_or_else(|| {
                                                chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
                                            })
                                    })
                                    .unwrap_or_else(|| {
                                        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
                                    });

                                // Extract content from message
                                let content = if let Some(content_array) = message.get("content").and_then(|c| c.as_array()) {
                                    // Join text blocks
                                    content_array
                                        .iter()
                                        .filter_map(|part| {
                                            part.get("text")
                                                .and_then(|t| t.as_str())
                                                .or_else(|| {
                                                    part.get("thinking")
                                                        .and_then(|t| t.as_str())
                                                })
                                        })
                                        .collect::<Vec<_>>()
                                        .join("\n")
                                } else {
                                    message.get("content")
                                        .and_then(|c| c.as_str())
                                        .unwrap_or("")
                                        .to_string()
                                };

                                let ws_event = WSEvent::MessageAdded {
                                    session_id: ws_id.clone(),
                                    role: role.to_string(),
                                    content,
                                    timestamp,
                                };
                                app_state.ws_state.broadcast(ws_event);
                            }
                        }
                        "agent_start" => {
                            // Agent started processing
                            println!("🤖 Agent started for session {}", ws_id);
                        }
                        "agent_end" => {
                            // Agent completed
                            println!("✅ Agent completed for session {}", ws_id);
                        }
                        "notify" => {
                            // Notification from pi (e.g., tools loaded)
                            let message = event.extra.get("message")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            println!("📢 Notification: {}", message);
                        }
                        "response" => {
                            // Response to a command
                            let success = event.extra.get("success")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(true);
                            if !success {
                                let error = event.extra.get("error")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("Unknown error");
                                println!("❌ Command failed: {}", error);
                            }
                        }
                        _ => {
                            println!("Unhandled event type: {}", event_type);
                        }
                    }
                }
            }
        }
    }

    println!("📡 Event bridge task ended");
}

async fn health_check() -> Json<Value> {
    Json(serde_json::json!({
        "status": "ok",
        "service": "pi-agent-manager",
        "version": "0.1.0"
    }))
}


