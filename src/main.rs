use axum::{
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

mod config;
mod sessions;
mod websocket;
mod pi;
mod api;
mod static_files;
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

/// Pika - Manages multiple agent sessions and their execution contexts
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
        .unwrap_or(3000);
    
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    
    // Create combined application state
    let app_state = AppState::new(config.clone());
    
    // Build our application with health check, WebSocket, and API routes
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/ws", get(websocket::ws_handler))
        .merge(api::create_api_router())
        .fallback(static_files::serve_static_files)
        .with_state(app_state.clone())
        .layer(
            // CORS layer for local development and production
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        );

    println!("🚀 Pika server listening on http://{}", addr);
    println!("📡 WebSocket endpoint available at ws://{}", addr);

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
                let ws_event = WSEvent::SessionStarted {
                    session_id: id.clone(),
                    project_path: project_path.to_string_lossy().to_string(),
                };
                app_state.ws_state.broadcast(ws_event);
            }
            ProcessManagerEvent::ProcessKilled { id } => {
                println!("🛑 Process killed: {}", id);
                let ws_event = WSEvent::SessionStopped {
                    session_id: id.clone(),
                };
                app_state.ws_state.broadcast(ws_event);
            }
            ProcessManagerEvent::SessionStarted { session_id, process_id } => {
                println!("🎯 Session started: {} (process: {})", session_id, process_id);
                // SessionStarted is already handled by ProcessSpawned, so this is redundant
                // but we keep it for future use if we need to distinguish between the two
            }
            ProcessManagerEvent::JsonRpc { id, event } => {
                println!("📨 JSON-RPC event from {}: {:?}", id, event);

                // Convert JsonRpcEvent to WSEvent
                // The pi process emits events with method names
                if let Some(method) = &event.method {
                    match method.as_str() {
                        "thinking" => {
                            // Thinking delta event
                            if let Some(params) = &event.params {
                                let content = params.get("content")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("");

                                let ws_event = WSEvent::ThinkingDelta {
                                    session_id: id.clone(),
                                    content: content.to_string(),
                                };
                                app_state.ws_state.broadcast(ws_event);
                            }
                        }
                        "message" => {
                            // New message added
                            if let Some(params) = &event.params {
                                let role = params.get("role")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("assistant");

                                let content = params.get("content")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("");

                                let timestamp = params.get("timestamp")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or(""); // Empty string if no timestamp provided

                                let ws_event = WSEvent::MessageAdded {
                                    session_id: id.clone(),
                                    role: role.to_string(),
                                    content: content.to_string(),
                                    timestamp: timestamp.to_string(),
                                };
                                app_state.ws_state.broadcast(ws_event);
                            }
                        }
                        _ => {
                            println!("Unhandled JSON-RPC method: {}", method);
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
        "service": "pika",
        "version": "0.1.0"
    }))
}


