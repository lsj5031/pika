use axum::{
    extract::{
        ConnectInfo, Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use futures::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use tracing::info;

use crate::api::types::ImageAttachmentResponse;
use crate::auth::is_request_authenticated;
use crate::{AppState, extract_client_ip};

/// Query parameters for WebSocket connection.
/// Legacy `auth` query parameter is intentionally blocked for security reasons.
#[derive(Debug, Deserialize)]
pub struct WsQueryParams {
    pub auth: Option<String>,
}

/// WebSocket events that can be broadcast to connected clients
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum WSEvent {
    /// A new session has started
    SessionStarted {
        session_id: String,
        project_path: String,
    },
    /// A session has stopped
    SessionStopped { session_id: String },
    /// Thinking update (delta for streaming)
    ThinkingDelta { session_id: String, content: String },
    /// A new message was added to the conversation
    MessageAdded {
        session_id: String,
        role: String,
        content: String,
        timestamp: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        images: Option<Vec<ImageAttachmentResponse>>,
    },
    /// An error occurred during agent execution or system operation
    Error {
        session_id: Option<String>,
        message: String,
        code: Option<String>,
    },
    /// Response to an RPC command (set_model, get_state, get_available_models, etc.)
    CommandResponse {
        session_id: String,
        command: String,
        success: bool,
        data: Option<serde_json::Value>,
        error: Option<String>,
    },
}

impl WSEvent {
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

/// Shared state for WebSocket connections
#[derive(Clone)]
pub struct WSState {
    /// Broadcast channel for sending events to all connected clients
    pub tx: broadcast::Sender<WSEvent>,
    /// Set of active client IDs (for connection tracking)
    pub clients: Arc<RwLock<std::collections::HashSet<String>>>,
}

impl WSState {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(1000);
        Self {
            tx,
            clients: Arc::new(RwLock::new(std::collections::HashSet::new())),
        }
    }

    /// Broadcast an event to all connected clients
    pub fn broadcast(&self, event: WSEvent) {
        let _ = self.tx.send(event);
    }

    /// Get the number of connected clients
    pub async fn client_count(&self) -> usize {
        self.clients.read().await.len()
    }
}

/// Default implementation for WSState
impl Default for WSState {
    fn default() -> Self {
        Self::new()
    }
}

/// Handle WebSocket upgrade and manage the connection.
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<WsQueryParams>,
    State(state): State<AppState>,
    ConnectInfo(peer_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Response {
    // Explicitly block legacy query-param auth.
    if params.auth.is_some() {
        return (
            StatusCode::BAD_REQUEST,
            "WebSocket query auth is disabled. Use session cookie auth.",
        )
            .into_response();
    }

    // Rate limit WS connection attempts per client IP.
    let client_ip = extract_client_ip(
        &headers,
        peer_addr,
        state.trusted_proxy_cidrs.as_ref().as_slice(),
    );
    let ws_decision = state
        .rate_limits
        .websocket_connect
        .check(&client_ip.to_string())
        .await;
    if !ws_decision.allowed {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            format!(
                "Too many WebSocket connection attempts. Retry in {}s.",
                ws_decision.retry_after_seconds
            ),
        )
            .into_response();
    }

    // Enforce same auth gate as HTTP API.
    if !is_request_authenticated(&headers, &state.auth_context) {
        return (StatusCode::UNAUTHORIZED, "Authentication required").into_response();
    }

    let ws_state = state.ws_state.clone();
    ws.on_upgrade(|socket| handle_socket(socket, ws_state))
}

/// Handle an individual WebSocket connection
async fn handle_socket(socket: WebSocket, state: WSState) {
    // Generate a unique ID for this client
    let client_id = uuid::Uuid::new_v4().to_string();

    // Add client to the active set
    {
        let mut clients = state.clients.write().await;
        clients.insert(client_id.clone());
    }

    let connected_clients = state.client_count().await;
    info!(clients = connected_clients, "WebSocket client connected");

    // Subscribe to the broadcast channel
    let mut rx = state.tx.subscribe();

    // Split the socket into sender and receiver
    let (mut sender, mut receiver) = socket.split();

    // Task to handle incoming messages from the client
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Close(_) => {
                    break;
                }
                Message::Ping(_) | Message::Pong(_) => {}
                Message::Text(_) => {
                    // Ignore client text payloads to avoid logging sensitive data.
                }
                _ => {}
            }
        }
    });

    // Task to handle outgoing messages to the client
    let send_task = tokio::spawn(async move {
        while let Ok(event) = rx.recv().await {
            if let Ok(json) = event.to_json()
                && sender.send(Message::Text(json.into())).await.is_err()
            {
                break;
            }
        }
    });

    // Wait for either task to complete
    tokio::select! {
        _ = recv_task => {}
        _ = send_task => {}
    }

    // Remove client from the active set
    {
        let mut clients = state.clients.write().await;
        clients.remove(&client_id);
    }

    let connected_clients = state.client_count().await;
    info!(clients = connected_clients, "WebSocket client disconnected");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ws_event_serialization() {
        let event = WSEvent::SessionStarted {
            session_id: "test-session".to_string(),
            project_path: "/path/to/project".to_string(),
        };
        let json = event.to_json().unwrap();
        assert!(json.contains("SessionStarted"));
        assert!(json.contains("test-session"));
    }

    #[tokio::test]
    async fn test_ws_state_creation() {
        let state = WSState::new();
        assert_eq!(state.client_count().await, 0);
    }

    #[test]
    fn test_ws_event_thinking_delta() {
        let event = WSEvent::ThinkingDelta {
            session_id: "session-123".to_string(),
            content: "Thinking...".to_string(),
        };
        let json = event.to_json().unwrap();
        assert!(json.contains("ThinkingDelta"));
        assert!(json.contains("Thinking..."));
    }

    #[test]
    fn test_ws_event_message_added() {
        let event = WSEvent::MessageAdded {
            session_id: "session-456".to_string(),
            role: "assistant".to_string(),
            content: "Hello!".to_string(),
            timestamp: "2026-01-13T00:00:00Z".to_string(),
            images: None,
        };
        let json = event.to_json().unwrap();
        assert!(json.contains("MessageAdded"));
        assert!(json.contains("assistant"));
        assert!(json.contains("Hello!"));
    }
}
