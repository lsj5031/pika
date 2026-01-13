use axum::{
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::Response,
};
use futures::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

use crate::AppState;

/// WebSocket events that can be broadcast to connected clients
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum WSEvent {
    /// A new session has started
    SessionStarted { session_id: String, project_path: String },
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
        let (tx, _) = broadcast::channel(100);
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

/// Handle WebSocket upgrade and manage the connection
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> Response {
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

    println!("📡 WebSocket client connected: {} (total: {})", 
        client_id, 
        state.client_count().await
    );

    // Subscribe to the broadcast channel
    let mut rx = state.tx.subscribe();

    // Split the socket into sender and receiver
    let (mut sender, mut receiver) = socket.split();

    // Task to handle incoming messages from the client
    let client_id_clone = client_id.clone();
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Close(_) => {
                    println!("📡 WebSocket client closing: {}", client_id_clone);
                    break;
                }
                Message::Ping(_data) => {
                    // Respond to pings
                    // Note: We'd need to send a Pong back, but with split socket this is tricky
                    // For now, we'll just log it
                }
                Message::Pong(_) => {
                    // Client responded to our ping
                }
                Message::Text(text) => {
                    // Handle text messages from client (if needed)
                    println!("📡 Received from {}: {}", client_id_clone, text);
                }
                _ => {}
            }
        }
    });

    // Task to handle outgoing messages to the client
    let client_id_for_send = client_id.clone();
    let send_task = tokio::spawn(async move {
        while let Ok(event) = rx.recv().await {
            if let Ok(json) = event.to_json() {
                if sender.send(Message::Text(json.into())).await.is_err() {
                    println!("📡 Failed to send to client: {}", client_id_for_send);
                    break;
                }
            }
        }
    });

    // Wait for either task to complete
    tokio::select! {
        _ = recv_task => {
            println!("📡 Receiver task ended for client: {}", client_id);
        }
        _ = send_task => {
            println!("📡 Sender task ended for client: {}", client_id);
        }
    }

    // Remove client from the active set
    {
        let mut clients = state.clients.write().await;
        clients.remove(&client_id);
    }

    println!("📡 WebSocket client disconnected: {} (total: {})", 
        client_id, 
        state.client_count().await
    );
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
        };
        let json = event.to_json().unwrap();
        assert!(json.contains("MessageAdded"));
        assert!(json.contains("assistant"));
        assert!(json.contains("Hello!"));
    }
}
