use axum::{
    Router,
    body::Body,
    extract::DefaultBodyLimit,
    http::{HeaderValue, Method, Request, Uri, header},
    middleware::{self, Next},
    response::{Json, Response},
    routing::get,
};
use clap::Parser;
use pika::{
    AppState, AuthContext, AuthCredentials, ProcessManagerEvent, ProjectConfig, RateLimitState,
    SessionCookieManager, SessionFileEvent, SessionFileWatcher, WSEvent, auth_middleware,
    build_encoded_project_map, build_session_index, create_api_router, create_auth_router,
    load_session_info_from_file, serve_static_files, ws_handler,
};
use serde_json::Value;
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use tokio::net::TcpListener;
use tower_http::cors::{AllowOrigin, CorsLayer};

/// Pika - Manages multiple agent sessions and their execution contexts
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Path to the configuration file (default: ./config.toml)
    #[arg(short, long, global = true)]
    config: Option<PathBuf>,

    /// Port to listen on (default: 7847, overrides PORT env var)
    #[arg(short, long)]
    port: Option<u16>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse CLI arguments
    let cli = Cli::parse();

    // Determine config file path (default: ./config.toml)
    let config_path = cli.config.unwrap_or_else(|| PathBuf::from("config.toml"));
    let config_path = if config_path.is_absolute() {
        config_path
    } else {
        std::env::current_dir()?.join(config_path)
    };

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

    let bind_address = config.get_bind_address();
    let bind_ip: IpAddr = bind_address.parse().map_err(|_| {
        format!(
            "Invalid bind address '{}'. Expected an IP address (e.g. 127.0.0.1).",
            bind_address
        )
    })?;
    let addr = SocketAddr::new(bind_ip, port);
    let is_localhost_bind = bind_ip.is_loopback();

    let auth_enabled = config.is_auth_enabled();
    let auth_disabled = config.is_auth_disabled();

    if !is_localhost_bind && !auth_enabled && !config.allow_insecure_remote_mode() {
        return Err(
            "Refusing to start: remote bind without authentication is blocked. \
Set AUTH_USERNAME/AUTH_PASSWORD or explicitly set ALLOW_INSECURE_REMOTE=true."
                .into(),
        );
    }

    if !auth_enabled && !auth_disabled {
        println!(
            "⚠️  Authentication is not enabled. Set AUTH_USERNAME and AUTH_PASSWORD to secure access."
        );
    }

    let auth_credentials = if auth_disabled {
        AuthCredentials::new(String::new(), String::new())
    } else {
        AuthCredentials::new(
            config.get_auth_username().unwrap_or_default(),
            config.get_auth_password().unwrap_or_default(),
        )
    };

    let session_secret = match config.get_session_secret() {
        Some(secret) => {
            if let Err(error) = ProjectConfig::validate_session_secret_strength(&secret) {
                if auth_enabled {
                    return Err(format!(
                        "Refusing to start with weak AUTH_SESSION_SECRET: {}",
                        error
                    )
                    .into());
                }

                println!("⚠️  Weak AUTH_SESSION_SECRET: {}", error);
            }

            secret
        }
        None => {
            println!(
                "⚠️  AUTH_SESSION_SECRET not set. Using ephemeral session secret (sessions invalidate on restart)."
            );
            uuid::Uuid::new_v4().to_string()
        }
    };

    let session_cookie_secure = config.session_cookie_secure();
    if auth_enabled && !session_cookie_secure {
        println!("⚠️  session_cookie_secure=false. Use this only for local HTTP development.");

        if !is_localhost_bind {
            return Err(
                "Refusing to start: insecure session cookies are not allowed on non-localhost binds."
                    .into(),
            );
        }
    }

    let auth_context = AuthContext::new(
        auth_credentials.clone(),
        auth_enabled,
        SessionCookieManager::new(
            session_secret.into_bytes(),
            config.session_ttl_seconds,
            session_cookie_secure,
        ),
    );

    let rate_limits = RateLimitState::new(
        config.login_rate_limit_per_minute,
        config.ws_connect_rate_limit_per_minute,
    );

    // Create combined application state
    let app_state = AppState::new(
        config.clone(),
        config_path.clone(),
        auth_context.clone(),
        rate_limits.clone(),
    );

    // Build in-memory session index for fast lookups
    let session_index = build_session_index(&config).await;
    {
        let mut index = app_state.session_index.write().await;
        *index = session_index;
    }

    // Build protected API routes (require signed session cookie if auth is enabled)
    let protected_routes = Router::new()
        .merge(create_api_router())
        .layer(middleware::from_fn(move |req, next| {
            let auth = auth_context.clone();
            auth_middleware(req, next, auth)
        }));

    let allowed_origins = resolve_allowed_origins(&config, port)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidInput, error))?;

    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list(allowed_origins))
        .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE])
        .allow_credentials(true);

    // Build the full application
    // - /health and /api/auth/* are public
    // - /ws and /api/* are protected by auth gates
    // - Static files are public for app bootstrap
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/ws", get(ws_handler))
        .fallback(serve_static_files)
        .merge(create_auth_router())
        .merge(protected_routes)
        .with_state(app_state.clone())
        .layer(DefaultBodyLimit::max(config.max_request_body_bytes))
        .layer(middleware::from_fn(security_headers_middleware))
        .layer(cors);

    println!("🚀 Pika server listening on http://{}", addr);
    println!("📡 WebSocket endpoint available at ws://{}", addr);
    println!("🌐 CORS allowlist configured");

    if auth_enabled {
        println!("🔐 Authentication enabled (session cookie required for protected routes)");
    } else if auth_disabled {
        println!("⚠️  Authentication disabled via debug override");
    } else {
        println!("⚠️  Authentication disabled (credentials missing)");
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

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}

fn resolve_allowed_origins(config: &ProjectConfig, port: u16) -> Result<Vec<HeaderValue>, String> {
    let configured = config.get_allowed_cors_origins();
    let using_defaults = configured.is_empty();

    let defaults = vec![
        format!("http://127.0.0.1:{}", port),
        format!("http://localhost:{}", port),
        "http://127.0.0.1:5173".to_string(),
        "http://localhost:5173".to_string(),
    ];

    let origins = if using_defaults { defaults } else { configured };

    let mut parsed = Vec::new();
    let mut invalid = Vec::new();

    for origin in origins {
        match origin.parse::<Uri>() {
            Ok(uri)
                if uri.scheme().is_some()
                    && uri.authority().is_some()
                    && uri.path() == "/"
                    && uri.query().is_none() =>
            {
                if let Ok(header_value) = origin.parse::<HeaderValue>() {
                    parsed.push(header_value);
                } else {
                    invalid.push(origin);
                }
            }
            _ => invalid.push(origin),
        }
    }

    if !invalid.is_empty() {
        println!("⚠️  Ignoring invalid CORS origins: {}", invalid.join(", "));
    }

    if parsed.is_empty() {
        if using_defaults {
            Err("No valid default CORS origins were generated".to_string())
        } else {
            Err("No valid configured CORS origins remain after validation".to_string())
        }
    } else {
        Ok(parsed)
    }
}

async fn security_headers_middleware(req: Request<Body>, next: Next) -> Response {
    let mut response = next.run(req).await;
    let headers = response.headers_mut();

    headers.insert("x-content-type-options", "nosniff".parse().unwrap());
    headers.insert("x-frame-options", "DENY".parse().unwrap());
    headers.insert("referrer-policy", "no-referrer".parse().unwrap());
    headers.insert(
        "content-security-policy",
        "default-src 'self'; base-uri 'self'; frame-ancestors 'none'; object-src 'none'; img-src 'self' data:; connect-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'"
            .parse()
            .unwrap(),
    );

    response
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
                                // Notification from pi (content intentionally not logged).
                                println!("📢 Notification received");
                            }
                            "response" => {
                                // Response to a command
                                let success = event
                                    .extra
                                    .get("success")
                                    .and_then(|v| v.as_bool())
                                    .unwrap_or(true);
                                if !success {
                                    println!("❌ Command failed");
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
