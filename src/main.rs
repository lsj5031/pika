use axum::{Router, middleware, response::Json, routing::get};
use clap::Parser;
use pika::{
    AppState, AuthCredentials, ProjectConfig, ProcessManagerEvent, WSEvent,
    SessionFileEvent, SessionFileWatcher,
    build_encoded_project_map, build_session_index, load_session_info_from_file,
    create_api_router, basic_auth_middleware, ws_handler, serve_static_files,
};
use serde_json::Value;
use std::net::SocketAddr;
use std::path::PathBuf;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};

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
        println!(
            "📁 Monitoring {} project root path(s):",
            config.project_root_paths.len()
        );
        for path in &config.project_root_paths {
            println!("   - {}", path.display());
        }
    }

    // Load port from CLI arg, environment variable, or use default
    let port = cli
        .port
        .or_else(|| std::env::var("PORT").ok().and_then(|p| p.parse().ok()))
        .unwrap_or(7847);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    // Create combined application state
    let app_state = AppState::new(config.clone());

    // Build in-memory session index for fast lookups
    let session_index = build_session_index(&config).await;
    {
        let mut index = app_state.session_index.write().await;
        *index = session_index;
    }

    // Set up auth credentials
    let auth_credentials = if config.is_auth_disabled() {
        AuthCredentials::new(String::new(), String::new())
    } else {
        AuthCredentials::new(
            config.get_auth_username().unwrap_or_default(),
            config.get_auth_password().unwrap_or_default(),
        )
    };
    let auth_enabled = config.is_auth_enabled();

    // Build protected API routes (require auth via HTTP header if enabled)
    let protected_routes = Router::new()
        .merge(create_api_router())
        .with_state(app_state.clone())
        .layer(middleware::from_fn(move |req, next| {
            let creds = auth_credentials.clone();
            basic_auth_middleware(req, next, creds)
        }));

    // Build the full application
    // - /health is always public
    // - /ws handles its own auth via query params (WebSocket doesn't support headers)
    // - Static files are public (the frontend itself must load to show auth UI)
    // - API routes are protected
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/ws", get(ws_handler))
        .fallback(serve_static_files)
        .with_state(app_state.clone())
        .merge(protected_routes)
        .layer(
            // CORS layer for local development and production
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        );

    println!("🚀 Pika server listening on http://{}", addr);
    println!("📡 WebSocket endpoint available at ws://{}", addr);
    if auth_enabled {
        println!("🔐 HTTP Basic Auth enabled");
    } else if config.is_auth_disabled() {
        println!("⚠️  HTTP Basic Auth disabled (debug override enabled)");
    } else {
        println!("⚠️  HTTP Basic Auth disabled (no credentials configured)");
    }

    let listener = TcpListener::bind(addr).await?;

    // Start event bridging task (ProcessManager -> WebSocket)
    let app_state_for_bridge = app_state.clone();
    tokio::spawn(async move {
        event_bridge_task(app_state_for_bridge).await;
    });

    // Build encoded project map for lossless path resolution
    let encoded_project_map = build_encoded_project_map(&config);

    // Start file watcher task for real-time session updates
    let app_state_for_watcher = app_state.clone();
    tokio::spawn(async move {
        file_watcher_task(app_state_for_watcher, encoded_project_map).await;
    });

    axum::serve(listener, app).await?;

    Ok(())
}

/// Background task that watches session files for changes and broadcasts WebSocket events
async fn file_watcher_task(
    app_state: AppState,
    encoded_project_map: std::collections::HashMap<String, std::path::PathBuf>,
) {
    use tokio::sync::broadcast::error::RecvError;

    // Create file watcher with the encoded project map
    let mut watcher = match SessionFileWatcher::new(encoded_project_map) {
        Ok(w) => w,
        Err(e) => {
            println!("⚠️ Failed to create file watcher: {}", e);
            return;
        }
    };

    // Start watching
    if let Err(e) = watcher.start_watching() {
        println!("⚠️ Failed to start file watcher: {}", e);
        return;
    }

    // Subscribe to file events
    let mut rx = watcher.subscribe();

    println!("📂 File watcher task started");

    loop {
        match rx.recv().await {
            Ok(event) => {
                match event {
                    SessionFileEvent::SessionFileCreated {
                        project_path,
                        session_id,
                        file_path,
                    } => {
                        println!(
                            "📁 New session file created: {} in {:?}",
                            session_id, project_path
                        );
                        // Notify frontend that sessions list should be refreshed
                        // We use SessionStarted event to trigger UI update
                        let ws_event = WSEvent::SessionStarted {
                            session_id: session_id.clone(),
                            project_path: project_path.to_string_lossy().to_string(),
                        };
                        app_state.ws_state.broadcast(ws_event);

                        // Also log the file path for debugging
                        println!("   File: {:?}", file_path);

                        if let Ok(session_info) =
                            load_session_info_from_file(&project_path, &file_path).await
                        {
                            let mut index = app_state.session_index.write().await;
                            index.upsert(session_info);
                        }
                    }
                    SessionFileEvent::SessionFileModified {
                        project_path,
                        session_id,
                        file_path,
                    } => {
                        // Session file was modified - this means new messages were added
                        // The frontend can poll for new messages or we could parse the diff
                        // For now, we just invalidate the messages cache
                        println!(
                            "📝 Session file modified: {} (in {})",
                            session_id,
                            project_path.display()
                        );
                        println!("   File: {:?}", file_path);

                        if let Ok(session_info) =
                            load_session_info_from_file(&project_path, &file_path).await
                        {
                            let mut index = app_state.session_index.write().await;
                            index.upsert(session_info);
                        }

                        // Note: We don't send MessageAdded here because the pi process
                        // already sends that event via JSON-RPC when it writes to the file.
                        // This watcher is mainly for catching external changes.
                    }
                    SessionFileEvent::SessionFileRemoved {
                        project_path,
                        session_id,
                        file_path,
                    } => {
                        println!(
                            "🗑️ Session file removed: {} (in {})",
                            session_id,
                            project_path.display()
                        );
                        println!("   File: {:?}", file_path);

                        let mut index = app_state.session_index.write().await;
                        index.remove(&session_id);
                    }
                }
            }
            Err(RecvError::Lagged(count)) => {
                eprintln!("⚠️ File watcher lagged, missed {} events", count);
                let config = app_state.api_state.config.read().await;
                let rebuilt = build_session_index(&config).await;
                let mut index = app_state.session_index.write().await;
                *index = rebuilt;
                continue;
            }
            Err(RecvError::Closed) => {
                break;
            }
        }
    }

    println!("📂 File watcher task ended");
}

/// Background task that bridges ProcessManager events to WebSocket events
async fn event_bridge_task(app_state: AppState) {
    use tokio::sync::broadcast::error::RecvError;

    let mut rx = {
        let pm = app_state.process_manager.lock().await;
        pm.subscribe()
    };

    println!("📡 Event bridge task started");

    loop {
        match rx.recv().await {
            Ok(event) => match event {
                ProcessManagerEvent::ProcessSpawned { id, project_path } => {
                    println!("🚀 Process spawned: {} in {}", id, project_path.display());

                    // Don't mark session as active yet - wait for agent_start event
                    // The process being spawned doesn't mean the agent is actively working
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

                    let ws_event = WSEvent::SessionStopped { session_id: ws_id };
                    app_state.ws_state.broadcast(ws_event);
                }
                ProcessManagerEvent::SessionStarted {
                    session_id,
                    process_id,
                } => {
                    println!(
                        "🎯 Session started: {} (process: {})",
                        session_id, process_id
                    );
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
                    // Pika sends events with "type" field
                    if let Some(event_type) = &event.event_type {
                        match event_type.as_str() {
                            "message_update" => {
                                // Streaming update - check if it's thinking or text
                                if let Some(msg_event) = event.extra.get("assistantMessageEvent")
                                    && let Some(delta_type) =
                                        msg_event.get("type").and_then(|t| t.as_str())
                                {
                                    match delta_type {
                                        "thinking_delta" => {
                                            let content = msg_event
                                                .get("delta")
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
                            "message_end" => {
                                // Message completed - extract role and content
                                if let Some(message) = event.extra.get("message") {
                                    let role = message
                                        .get("role")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("assistant");

                                    // Get timestamp
                                    let timestamp = message
                                        .get("timestamp")
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
                                                .map(|dt| {
                                                    dt.format("%Y-%m-%dT%H:%M:%SZ").to_string()
                                                })
                                                .unwrap_or_else(|| {
                                                    chrono::Utc::now()
                                                        .format("%Y-%m-%dT%H:%M:%SZ")
                                                        .to_string()
                                                })
                                        })
                                        .unwrap_or_else(|| {
                                            chrono::Utc::now()
                                                .format("%Y-%m-%dT%H:%M:%SZ")
                                                .to_string()
                                        });

                                    // Extract content from message (matching sessions.rs format)
                                    let content = if let Some(content_array) =
                                        message.get("content").and_then(|c| c.as_array())
                                    {
                                        // Extract thinking blocks first
                                        let thinking_parts: Vec<String> = content_array
                                            .iter()
                                            .filter_map(|part| {
                                                if part.get("type").and_then(|t| t.as_str())
                                                    == Some("thinking")
                                                {
                                                    part.get("thinking")
                                                        .and_then(|t| t.as_str())
                                                        .filter(|s| !s.is_empty())
                                                        .map(|s| {
                                                            format!("<thinking>{}</thinking>", s)
                                                        })
                                                } else {
                                                    None
                                                }
                                            })
                                            .collect();

                                        // Extract text parts
                                        let text_parts: Vec<String> = content_array
                                            .iter()
                                            .filter_map(|part| {
                                                part.get("text")
                                                    .and_then(|t| t.as_str())
                                                    .map(|s| s.to_string())
                                            })
                                            .collect();

                                        // Combine thinking and text
                                        let mut all_parts = thinking_parts;
                                        all_parts.extend(text_parts);

                                        if !all_parts.is_empty() {
                                            all_parts.join("\n")
                                        } else {
                                            // Try tool_use / tool_result patterns
                                            let tool_parts: Vec<String> = content_array
                                                .iter()
                                                .filter_map(|part| {
                                                    if let Some(tool_use) = part
                                                        .get("tool_use")
                                                        .and_then(|t| t.as_object())
                                                    {
                                                        let name = tool_use
                                                            .get("name")
                                                            .and_then(|n| n.as_str())
                                                            .unwrap_or("unknown_tool");
                                                        let input = tool_use
                                                            .get("input")
                                                            .map(|i| {
                                                                if i.is_string() {
                                                                    i.as_str()
                                                                        .unwrap_or("")
                                                                        .to_string()
                                                                } else {
                                                                    serde_json::to_string(i)
                                                                        .unwrap_or_default()
                                                                }
                                                            })
                                                            .unwrap_or_default();
                                                        Some(format!(
                                                            "Tool Call: {}({})",
                                                            name, input
                                                        ))
                                                    } else if let Some(tool_result) = part
                                                        .get("tool_result")
                                                        .and_then(|t| t.as_object())
                                                    {
                                                        let is_error = tool_result
                                                            .get("is_error")
                                                            .and_then(|e| e.as_bool())
                                                            .unwrap_or(false);
                                                        let content = tool_result
                                                            .get("content")
                                                            .map(|c| {
                                                                if c.is_string() {
                                                                    c.as_str()
                                                                        .unwrap_or("")
                                                                        .to_string()
                                                                } else {
                                                                    serde_json::to_string(c)
                                                                        .unwrap_or_default()
                                                                }
                                                            })
                                                            .unwrap_or_default();
                                                        Some(format!(
                                                            "Tool Result{}: {}",
                                                            if is_error { " (Error)" } else { "" },
                                                            content
                                                        ))
                                                    } else {
                                                        None
                                                    }
                                                })
                                                .collect();

                                            if !tool_parts.is_empty() {
                                                tool_parts.join("\n")
                                            } else {
                                                format!(
                                                    "Tool call: {}",
                                                    serde_json::to_string(content_array)
                                                        .unwrap_or_default()
                                                )
                                            }
                                        }
                                    } else {
                                        message
                                            .get("content")
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
                                // Agent started processing - mark session as active
                                println!("🤖 Agent started for session {}", ws_id);
                                let ws_event = WSEvent::SessionStarted {
                                    session_id: ws_id.clone(),
                                    project_path: "".to_string(), // Not used for this purpose
                                };
                                app_state.ws_state.broadcast(ws_event);
                            }
                            "agent_end" => {
                                // Agent completed - mark session as inactive
                                println!("✅ Agent completed for session {}", ws_id);
                                let ws_event = WSEvent::SessionStopped {
                                    session_id: ws_id.clone(),
                                };
                                app_state.ws_state.broadcast(ws_event);
                            }
                            "notify" => {
                                // Notification from pi (e.g., tools loaded)
                                let message = event
                                    .extra
                                    .get("message")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("");
                                println!("📢 Notification: {}", message);
                            }
                            "response" => {
                                // Response to a command
                                let success = event
                                    .extra
                                    .get("success")
                                    .and_then(|v| v.as_bool())
                                    .unwrap_or(true);
                                if !success {
                                    let error = event
                                        .extra
                                        .get("error")
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
            },
            Err(RecvError::Lagged(count)) => {
                eprintln!("⚠️ Event bridge lagged, missed {} events", count);
                continue;
            }
            Err(RecvError::Closed) => {
                break;
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


