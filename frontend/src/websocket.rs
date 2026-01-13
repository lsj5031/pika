use makepad_widgets::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use crossbeam_channel::{Receiver, Sender, unbounded};

#[cfg(not(target_arch = "wasm32"))]
use tokio::task::JoinHandle;

/// WebSocket events from the backend
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

/// Action for WebSocket events
#[derive(Debug, Clone, DefaultNone)]
pub enum WSAction {
    None,
    Connected,
    Disconnected,
    EventReceived(WSEvent),
    Error(String),
}



/// WebSocket manager for all platforms
pub struct WSManager {
    #[cfg(not(target_arch = "wasm32"))]
    _handle: Option<JoinHandle<()>>,

    #[cfg(target_arch = "wasm32")]
    _handle: Option<()>,

    is_connected: Arc<AtomicBool>,
    event_receiver: Receiver<WSMessage>,
}

/// Messages from the WebSocket thread to the UI thread
pub enum WSMessage {
    Connected,
    Disconnected,
    Event(WSEvent),
    Error(String),
}

impl WSManager {
    /// Create a new WebSocket manager and start the connection
    /// This spawns a background task that maintains the WebSocket connection
    /// and sends events through a channel
    pub fn start(_cx: &mut Cx, ws_url: String) -> Self {
        log!("Starting WebSocket manager for: {}", ws_url);

        let is_connected = Arc::new(AtomicBool::new(false));
        let is_connected_clone = is_connected.clone();

        // Create a channel for communication between WebSocket task and UI thread
        let (event_sender, event_receiver) = unbounded::<WSMessage>();

        // Spawn a background task to handle WebSocket connection
        #[cfg(not(target_arch = "wasm32"))]
        let handle = Some(tokio::spawn(async move {
            Self::websocket_loop(ws_url, is_connected_clone, event_sender).await;
        }));

        #[cfg(target_arch = "wasm32")]
        let handle = None;

        Self {
            _handle: handle,
            is_connected,
            event_receiver,
        }
    }

    /// Check if the WebSocket is connected
    pub fn is_connected(&self) -> bool {
        self.is_connected.load(Ordering::Relaxed)
    }

    /// Poll for new events from the WebSocket
    /// This should be called regularly (e.g., in handle_event)
    /// Returns Some(event) if a new event is available, None otherwise
    pub fn poll_event(&self) -> Option<WSMessage> {
        self.event_receiver.try_recv().ok()
    }

    /// WebSocket connection loop with reconnection
    async fn websocket_loop(
        ws_url: String,
        is_connected: Arc<AtomicBool>,
        event_sender: Sender<WSMessage>,
    ) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            use futures_util::{SinkExt, StreamExt};
            use tokio::time::{sleep, Duration};
            use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

            let mut reconnect_delay = Duration::from_secs(1);

            loop {
                // Attempt to connect
                match connect_async(&ws_url).await {
                    Ok((ws_stream, _)) => {
                        log!("WebSocket connected to: {}", ws_url);
                        is_connected.store(true, Ordering::Relaxed);
                        reconnect_delay = Duration::from_secs(1); // Reset delay

                        // Send connected event
                        let _ = event_sender.send(WSMessage::Connected);

                        let (mut _write, mut read) = ws_stream.split();

                        // Handle incoming messages
                        loop {
                            match read.next().await {
                                Some(Ok(Message::Text(text))) => {
                                    log!("WebSocket received: {}", text);

                                    // Parse the event
                                    if let Ok(event) = serde_json::from_str::<WSEvent>(&text) {
                                        log!("Parsed WebSocket event: {:?}", event);

                                        // Send event through channel
                                        let _ = event_sender.send(WSMessage::Event(event));
                                    } else {
                                        log!("Failed to parse WebSocket event: {}", text);
                                    }
                                }
                                Some(Ok(Message::Close(_))) => {
                                    log!("WebSocket connection closed by server");
                                    break;
                                }
                                Some(Err(e)) => {
                                    log!("WebSocket error: {}", e);
                                    let _ = event_sender.send(WSMessage::Error(e.to_string()));
                                    break;
                                }
                                None => {
                                    log!("WebSocket stream ended");
                                    break;
                                }
                                _ => {}
                            }
                        }

                        // Mark as disconnected
                        is_connected.store(false, Ordering::Relaxed);
                        let _ = event_sender.send(WSMessage::Disconnected);

                        // Reconnect with exponential backoff
                        log!("WebSocket disconnected, reconnecting in {:?}", reconnect_delay);
                        sleep(reconnect_delay).await;
                        reconnect_delay = std::cmp::min(reconnect_delay * 2, Duration::from_secs(30));
                    }
                    Err(e) => {
                        log!("WebSocket connection error: {}", e);
                        is_connected.store(false, Ordering::Relaxed);
                        let _ = event_sender.send(WSMessage::Error(e.to_string()));

                        log!("Reconnecting in {:?}", reconnect_delay);
                        sleep(reconnect_delay).await;
                        reconnect_delay = std::cmp::min(reconnect_delay * 2, Duration::from_secs(30));
                    }
                }
            }
        }

        #[cfg(target_arch = "wasm32")]
        {
            log!("WASM WebSocket - not yet implemented");
            // For WASM, we would use the browser's WebSocket API
            // This is a placeholder for future implementation
            // Use pending() to create an infinite async loop
            loop {
                futures::future::pending::<()>().await;
            }
        }
    }
}
