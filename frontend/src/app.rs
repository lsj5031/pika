use makepad_widgets::*;
use crate::sessions::SessionAction;
use crate::HistoryAction;
use crate::sessions::chat_input::ChatInputAction;
use crate::websocket::{WSManager, WSAction, WSEvent, WSMessage};
use crate::sessions::Message;

#[cfg(not(target_arch = "wasm32"))]
use tokio::spawn;

// Action for async responses from HTTP fetch
#[derive(Debug, Clone, DefaultNone)]
pub enum AppAction {
    None,
    HealthCheck(String),
    Error(String),
    Session(SessionAction),
    WebSocket(WSAction),
    ChatInput(ChatInputAction),
    SessionStarted(String),
    SessionStartFailed(String),
    SessionStopped(String),
    CreateSession(String, String),
    NewSessionClicked,
    StopCurrentSession,
}

live_design! {
    import makepad_widgets::root::*;
    import makepad_widgets::theme::*;

    App = {{App}} {
        ui: <Root> {
            main_window = <Window> {
                body = <View> {
                    // Dark theme background
                    show_bg: true
                    draw_bg: { color: #1a1a1a }

                    flow: Right,
                    width: Fill,
                    height: Fill,

                    // Sidebar (300px width, acts as full drawer on mobile)
                    sidebar = <View> {
                        width: 300
                        height: Fill
                        flow: Down
                        padding: 20
                        spacing: 16

                        // Sidebar dark background with shadow
                        show_bg: true
                        draw_bg: { color: #242424 }

                        // Sidebar header with hamburger menu
                        <View> {
                            width: Fill
                            height: Fit
                            flow: Right
                            spacing: 12
                            align: { y: 0.5 }

                            hamburger_button = <Button> {
                                width: 44
                                height: 44
                                text: "☰"
                                draw_text: {
                                    color: #e0e0e0
                                    text_style: { font_size: 20.0 }
                                }
                                draw_bg: {
                                    color: #333
                                    color_hover: #444
                                    border_radius: 8.0
                                }
                            }

                            <Label> {
                                text: "Pika"
                                draw_text: {
                                    color: #e0e0e0
                                    text_style: { font_size: 18.0, font_weight: 600.0 }
                                }
                            }
                        }

                        // New Session button
                        new_session_button = <Button> {
                            width: Fill
                            height: 44
                            text: "+ New Session"
                            draw_text: {
                                color: #e0e0e0
                                text_style: { font_size: 16.0, font_weight: 500.0 }
                            }
                            draw_bg: {
                                color: #3b82f6
                                color_hover: #2b6fd6
                                border_radius: 8.0
                            }
                        }

                        // Session list
                        <SessionList> {
                            width: Fill
                            height: Fill
                        }

                        // Status section at bottom
                        <View> {
                            width: Fill
                            height: Fill
                            align: { y: 1.0 }

                            <View> {
                                width: Fill
                                height: Fit
                                flow: Down
                                spacing: 8
                                padding: { bottom: 12 }

                                <Label> {
                                    text: "Backend Status"
                                    draw_text: {
                                        color: #888
                                        text_style: { font_size: 14.0, font_weight: 500.0 }
                                    }
                                }

                                status_label = <Label> {
                                    text: "Connecting..."
                                    draw_text: {
                                        color: #666
                                        text_style: { font_size: 13.0 }
                                    }
                                }
                            }
                        }
                    }

                    // Main Panel (70%) - Session History and Chat Input
                    main_panel = <View> {
                        width: Fill
                        height: Fill
                        flow: Down

                        // Session header with stop button
                        <View> {
                            width: Fill
                            height: Fit
                            padding: 16
                            spacing: 12
                            flow: Right

                            show_bg: true
                            draw_bg: { color: #242424 }

                            session_info_label = <Label> {
                                width: Fill
                                text: "No session selected"
                                draw_text: {
                                    color: #888
                                    text_style: { font_size: 14.0, font_weight: 500.0 }
                                }
                            }

                            stop_session_button = <Button> {
                                width: Fit
                                height: 36
                                min_width: 100
                                visible: false
                                text: "Stop Session"
                                draw_text: {
                                    color: #ffffff
                                    text_style: { font_size: 14.0, font_weight: 600.0 }
                                }
                                draw_bg: {
                                    color: #dc2626
                                    color_hover: #b91c1c
                                    border_radius: 6.0
                                }
                            }
                        }

                        history_view = <SessionHistory> {
                            width: Fill
                            height: Fill
                        }

                        chat_input = <ChatInput> {
                            width: Fill
                            height: Fit
                        }
                    }
                }
            }
        }
    }
}

#[derive(Live, LiveHook, Default)]
pub struct App {
    #[live]
    ui: WidgetRef,

    #[rust]
    backend_url: String,

    #[rust]
    http_client: Option<reqwest::Client>,

    #[rust]
    sidebar_collapsed: bool,

    #[rust]
    ws_manager: Option<WSManager>,

    #[rust]
    current_session_id: Option<String>,

    #[rust]
    current_session_active: bool,

    #[rust]
    thinking_active: bool,

    #[rust]
    starting_session_id: Option<String>,

    #[rust]
    window_width: f64,

    #[rust]
    window_height: f64,
}

impl LiveRegister for App {
    fn live_register(cx: &mut Cx) {
        makepad_widgets::live_design(cx);
        crate::app::live_design(cx);
    }
}

impl MatchEvent for App {
    fn handle_startup(&mut self, cx: &mut Cx) {
        log!("Pika Frontend Started");

        // Get backend URL from environment or use default
        self.backend_url = std::env::var("BACKEND_URL")
            .unwrap_or_else(|_| "http://localhost:8765".to_string());

        log!("Backend URL: {}", self.backend_url);

        // Initialize HTTP client
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.http_client = Some(reqwest::Client::new());
        }

        // Initialize window size from current window
        let size = cx.default_window_size();
        self.window_width = size.x;
        self.window_height = size.y;
        log!("Initial window size: {}x{}", size.x, size.y);

        // Initialize mobile layout on startup
        self.sidebar_collapsed = self.is_mobile(); // Auto-collapse on mobile
        self.update_mobile_layout(cx);

        // Connect to WebSocket
        // Convert HTTP URL to WS URL
        let ws_url = self.backend_url.replace("http://", "ws://").replace("https://", "wss://");
        let ws_url = format!("{}/ws", ws_url);
        log!("WebSocket URL: {}", ws_url);

        let ws_manager = WSManager::start(cx, ws_url);
        self.ws_manager = Some(ws_manager);

        // Automatically fetch health check on startup
        self.fetch_backend_health(cx);

        // SessionList will auto-fetch on its own startup
    }

    fn handle_actions(&mut self, cx: &mut Cx, actions: &Actions) {
        // Handle hamburger button click
        if self.ui.button(id!(hamburger_button)).clicked(&actions) {
            self.sidebar_collapsed = !self.sidebar_collapsed;
            log!("Sidebar toggled: collapsed={}", self.sidebar_collapsed);

            // Update sidebar visibility based on mobile mode
            self.update_mobile_layout(cx);
        }

        // Handle stop session button click
        if self.ui.button(id!(stop_session_button)).clicked(&actions) {
            if let Some(session_id) = &self.current_session_id {
                log!("Stop session button clicked for: {}", session_id);

                // Check if session has active work (thinking)
                let has_active_work = self.thinking_active;

                if has_active_work {
                    // Session has active work - confirm before stopping
                    // For now, we'll just log a warning and proceed
                    // In a full implementation, this would show a confirmation dialog
                    log!("WARNING: Session {} has active work but user requested stop", session_id);
                }

                // Call the stop API
                let backend_url = self.backend_url.clone();
                let session_id_clone = session_id.clone();
                let client = self.http_client.clone();
                #[cfg(not(target_arch = "wasm32"))]
                spawn(async move {
                    let stop_url = format!("{}/api/sessions/{}/stop", backend_url, session_id_clone);
                    if let Some(client) = client {
                        match client.post(&stop_url).send().await {
                            Ok(response) => {
                                if response.status().is_success() {
                                    log!("Successfully sent stop request for session {}", session_id_clone);
                                    // WebSocket will handle the SessionStopped event to update UI
                                } else {
                                    log!("Failed to stop session {}: HTTP {}", session_id_clone, response.status());
                                }
                            }
                            Err(e) => {
                                log!("Failed to stop session {}: {}", session_id_clone, e);
                            }
                        }
                    }
                });
                #[cfg(target_arch = "wasm32")]
                log!("Stop session not implemented for WASM yet");
            }
        }

        // Handle new session button click
        if self.ui.button(id!(new_session_button)).clicked(&actions) {
            log!("New session button clicked");

            // For simplicity, create a session in the first available project
            // The backend will generate a timestamp-based name if we don't provide one
            let backend_url = self.backend_url.clone();
            let client = self.http_client.clone();

            #[cfg(not(target_arch = "wasm32"))]
            spawn(async move {
                // First, get the list of projects to find the first one
                let projects_url = format!("{}/api/projects", backend_url);

                if let Some(client) = client {
                    match client.get(&projects_url).send().await {
                        Ok(response) => {
                            if let Ok(projects) = response.json::<serde_json::Value>().await {
                                // Get the first project's ID
                                if let Some(first_project) = projects.as_array().and_then(|arr| arr.first()) {
                                    if let Some(project_id) = first_project.get("id").and_then(|id| id.as_str()) {
                                        log!("Creating new session in project: {}", project_id);

                                        // Call POST /api/projects/:id/sessions
                                        let create_url = format!("{}/api/projects/{}/sessions", backend_url, project_id);

                                        // Send request with empty name (backend will use timestamp)
                                        let create_body = serde_json::json!({ "name": null });

                                        match client
                                            .post(&create_url)
                                            .json(&create_body)
                                            .send()
                                            .await
                                        {
                                            Ok(create_response) => {
                                                if let Ok(session_data) = create_response.json::<serde_json::Value>().await {
                                                    if let Some(session_id) = session_data.get("session_id").and_then(|id| id.as_str()) {
                                                        log!("New session created with ID: {}", session_id);

                                                        // Emit action to select the new session
                                                        Cx::post_action(AppAction::Session(SessionAction::SessionClicked(session_id.to_string())));

                                                        // Refresh the session list to show the new session
                                                        Cx::post_action(SessionAction::SessionsLoaded(vec![]));
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                log!("Failed to create session: {}", e);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            log!("Failed to fetch projects: {}", e);
                        }
                    }
                }
            });
            #[cfg(target_arch = "wasm32")]
            log!("New session creation not implemented for WASM yet");
        }

        // Handle app actions (HTTP responses, WebSocket events, etc.)
        for action in actions {
            if let Some(app_action) = action.downcast_ref::<AppAction>() {
                match app_action {
                    AppAction::HealthCheck(status) => {
                        log!("Health check result: {}", status);
                        self.ui.label(id!(status_label))
                            .set_text(cx, &format!("✓ Connected"));
                        self.ui.redraw(cx);
                    }
                    AppAction::Error(err) => {
                        log!("Error: {}", err);
                        self.ui.label(id!(status_label))
                            .set_text(cx, &format!("✗ Error"));
                        self.ui.redraw(cx);
                    }
                    AppAction::Session(session_action) => {
                        log!("Session action: {:?}", session_action);
                        match session_action {
                            SessionAction::SessionClicked(session_id) => {
                                log!("Session clicked: {}", session_id);
                                self.current_session_id = Some(session_id.clone());

                                // Reset active state until we verify the session status
                                self.current_session_active = false;
                                self.update_stop_button_visibility(cx);
                                self.update_chat_input_state(cx);

                                // Find the session to get its name and active status
                                let session_name = {
                                    // Get sessions from SessionList
                                    // For now, use a default name
                                    "Session".to_string()
                                };

                                // Update session info label
                                self.ui.label(id!(session_info_label))
                                    .set_text(cx, &format!("Session: {}", session_name));

                                // Set starting state
                                self.starting_session_id = Some(session_id.clone());

                                // Emit loading state to SessionList
                                cx.action(SessionAction::SessionLoading { session_id: Some(session_id.clone()) });

                                // Emit action to SessionHistory to load this session immediately
                                cx.action(HistoryAction::SessionChanged(session_id.clone()));

                                // Note: ChatInput will automatically use the current session when sending messages

                                // Check if session is running and start if needed
                                let session_id_clone = session_id.clone();
                                let backend_url = self.backend_url.clone();
                                let client = self.http_client.clone();

                                #[cfg(not(target_arch = "wasm32"))]
                                {
                                    tokio::spawn(async move {
                                        // Check session status first
                                        let status_url = format!("{}/api/sessions/{}/status", backend_url, session_id_clone);

                                        if let Some(client) = client {
                                            match client.get(&status_url).send().await {
                                                Ok(response) => {
                                                    if let Ok(status_data) = response.json::<serde_json::Value>().await {
                                                        let is_running = status_data.get("is_running")
                                                            .and_then(|v| v.as_bool())
                                                            .unwrap_or(false);

                                                        log!("Session {} is_running: {}", session_id_clone, is_running);

                                                        if !is_running {
                                                            // Start the session
                                                            log!("Starting session {}...", session_id_clone);
                                                            let start_url = format!("{}/api/sessions/{}/start", backend_url, session_id_clone);

                                                            match client.post(&start_url).send().await {
                                                                Ok(_) => {
                                                                    log!("Session {} started successfully", session_id_clone);
                                                                    Cx::post_action(AppAction::SessionStarted(session_id_clone.clone()));
                                                                }
                                                                Err(e) => {
                                                                    log!("Failed to start session {}: {}", session_id_clone, e);
                                                                    Cx::post_action(AppAction::SessionStartFailed(session_id_clone.clone()));
                                                                }
                                                            }
                                                    } else {
                                                        log!("Session {} already running", session_id_clone);
                                                        // Emit SessionStarted even if already running to clear loading state
                                                        Cx::post_action(AppAction::SessionStarted(session_id_clone.clone()));
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                log!("Failed to check session status for {}: {}", session_id_clone, e);
                                                // Emit SessionStartFailed to clear loading state
                                                Cx::post_action(AppAction::SessionStartFailed(session_id_clone.clone()));
                                            }
                                        }
                                        } else {
                                            log!("HTTP client not initialized, cannot check session status");
                                            Cx::post_action(AppAction::SessionStartFailed(session_id_clone.clone()));
                                        }
                                    });
                                }
                            }
                            _ => {}
                        }
                    }
                    AppAction::ChatInput(chat_action) => {
                        match chat_action {
                            ChatInputAction::MessageSent { content } => {
                                log!("Message sent: {}", content);

                                // Optimistic update: add the user message to SessionHistory immediately
                                if let Some(session_id) = &self.current_session_id {
                                    let message = Message {
                                        role: "user".to_string(),
                                        content: content.clone(),
                                        timestamp: Some({
                                            // Use current time
                                            use std::time::{SystemTime, UNIX_EPOCH};
                                            let duration = SystemTime::now()
                                                .duration_since(UNIX_EPOCH)
                                                .unwrap();
                                            format!("{:.3?}", duration)
                                        }),
                                        thinking: false,
                                    };

                                    // Append to SessionHistory immediately
                                    cx.action(HistoryAction::MessageAppended(message));
                                }
                            }
                            ChatInputAction::SetBusy { busy } => {
                                // Update thinking state
                                self.thinking_active = *busy;
                                self.update_chat_input_state(cx);
                            }
                            ChatInputAction::SetSession { session_id } => {
                                // Update current session
                                self.current_session_id = Some(session_id.clone());
                                log!("Chat input session set to: {}", session_id);
                            }
                            ChatInputAction::None => {}
                        }
                    }
                    AppAction::SessionStarted(session_id) => {
                        log!("Session started successfully: {}", session_id);
                        self.starting_session_id = None;

                        // Clear loading state in SessionList
                        cx.action(SessionAction::SessionLoading { session_id: None });

                        // Update current_session_active if this is the current session
                        if let Some(current_id) = &self.current_session_id {
                            if current_id == session_id {
                                self.current_session_active = true;
                                self.update_stop_button_visibility(cx);
                                self.update_chat_input_state(cx);
                            }
                        }
                    }
                    AppAction::SessionStartFailed(session_id) => {
                        log!("Session start failed: {}", session_id);
                        self.starting_session_id = None;

                        // Clear loading state in SessionList
                        cx.action(SessionAction::SessionLoading { session_id: None });
                    }
                    AppAction::SessionStopped(session_id) => {
                        log!("Session stopped: {}", session_id);

                        // Update current_session_active if this is the current session
                        if let Some(current_id) = &self.current_session_id {
                            if current_id == session_id {
                                self.current_session_active = false;
                                self.update_stop_button_visibility(cx);
                                self.update_chat_input_state(cx);
                            }
                        }
                    }
                    AppAction::WebSocket(ws_action) => {
                        self.handle_websocket_action(cx, ws_action);
                    }
                    AppAction::CreateSession(project_id, name) => {
                        log!("App action: CreateSession in project {}, name: {:?}", project_id, name);
                        // This action is emitted when a new session is created
                        // The actual creation is handled by the frontend code that emits this action
                    }
                    AppAction::NewSessionClicked => {
                        log!("App action: NewSessionClicked");
                        // User clicked the new session button
                        // This is handled by the SessionList component
                    }
                    AppAction::StopCurrentSession => {
                        log!("App action: StopCurrentSession");
                        // User clicked the stop button
                        // Emit stop action to backend
                        if let Some(session_id) = &self.current_session_id {
                            let session_id_clone = session_id.clone();
                            let backend_url = self.backend_url.clone();
                            let client = self.http_client.clone();

                            #[cfg(not(target_arch = "wasm32"))]
                            {
                                tokio::spawn(async move {
                                    let stop_url = format!("{}/api/sessions/{}/stop", backend_url, session_id_clone);

                                    if let Some(client) = client {
                                        match client.post(&stop_url).send().await {
                                            Ok(_) => {
                                                log!("Session {} stopped successfully", session_id_clone);
                                            }
                                            Err(e) => {
                                                log!("Failed to stop session {}: {}", session_id_clone, e);
                                            }
                                        }
                                    }
                                });
                            }
                        }
                    }
                    AppAction::ChatInput(chat_input_action) => {
                        log!("App action: ChatInput {:?}", chat_input_action);
                        // Forward chat input actions to the chat input component
                        cx.action(chat_input_action.clone());
                    }
                    AppAction::SessionStarted(session_id) => {
                        log!("App action: SessionStarted {}", session_id);

                        // Update current session state if this is the active session
                        if let Some(current_id) = &self.current_session_id {
                            if current_id == session_id {
                                self.current_session_active = true;
                                self.update_stop_button_visibility(cx);
                                self.update_chat_input_state(cx);
                            }
                        }

                        // Clear loading state
                        self.starting_session_id = None;

                        // Emit to SessionList to update loading state
                        cx.action(SessionAction::SessionStarted { session_id: session_id.clone() });
                    }
                    AppAction::SessionStartFailed(session_id) => {
                        log!("App action: SessionStartFailed {}", session_id);

                        // Clear loading state
                        self.starting_session_id = None;

                        // Note: SessionAction doesn't have SessionStartFailed variant
                        // The loading state is cleared, which is sufficient feedback
                    }
                    AppAction::SessionStopped(session_id) => {
                        log!("App action: SessionStopped {}", session_id);

                        // Update current session state if this is the active session
                        if let Some(current_id) = &self.current_session_id {
                            if current_id == session_id {
                                self.current_session_active = false;
                                self.update_stop_button_visibility(cx);
                                self.update_chat_input_state(cx);
                            }
                        }

                        // Emit to SessionList to update state
                        cx.action(SessionAction::SessionStatusChanged {
                            session_id: session_id.clone(),
                            is_active: false,
                        });
                    }
                    AppAction::None => {}
                }
            }
        }
    }
}

impl App {
    /// Handle WebSocket actions (Connected, Disconnected, EventReceived, Error)
    fn handle_websocket_action(&mut self, cx: &mut Cx, action: &WSAction) {
        match action {
            WSAction::Connected => {
                log!("WebSocket action: Connected");
                self.ui.label(id!(status_label))
                    .set_text(cx, "✓ WS Connected");
                self.ui.redraw(cx);
            }
            WSAction::Disconnected => {
                log!("WebSocket action: Disconnected");
                self.ui.label(id!(status_label))
                    .set_text(cx, "○ WS Reconnecting...");
                self.ui.redraw(cx);
            }
            WSAction::EventReceived(ws_event) => {
                log!("WebSocket action: EventReceived");
                self.handle_websocket_event(cx, ws_event);
            }
            WSAction::Error(err) => {
                log!("WebSocket action: Error: {}", err);
                self.ui.label(id!(status_label))
                    .set_text(cx, &format!("✗ WS Error"));
                self.ui.redraw(cx);
            }
            WSAction::None => {}
        }
    }

    /// Handle WebSocket events from the backend (SessionStarted, SessionStopped, ThinkingDelta, MessageAdded)
    fn handle_websocket_event(&mut self, cx: &mut Cx, event: &WSEvent) {
        match event {
            WSEvent::SessionStarted { session_id, project_path } => {
                log!("WebSocket event: SessionStarted {} (path: {})", session_id, project_path);

                // Update session list with new active state
                cx.action(SessionAction::SessionStatusChanged {
                    session_id: session_id.clone(),
                    is_active: true,
                });

                // Update current session state if this is the active session
                if let Some(current_id) = &self.current_session_id {
                    if current_id == session_id {
                        self.current_session_active = true;
                        self.update_stop_button_visibility(cx);
                        self.update_chat_input_state(cx);
                    }
                }
            }
            WSEvent::SessionStopped { session_id } => {
                log!("WebSocket event: SessionStopped {}", session_id);

                // Update session list with inactive state
                cx.action(SessionAction::SessionStatusChanged {
                    session_id: session_id.clone(),
                    is_active: false,
                });

                // Update current session state if this is the active session
                if let Some(current_id) = &self.current_session_id {
                    if current_id == session_id {
                        self.current_session_active = false;
                        self.update_stop_button_visibility(cx);
                        self.update_chat_input_state(cx);
                    }
                }
            }
            WSEvent::ThinkingDelta { session_id, content } => {
                log!("WebSocket event: ThinkingDelta for session {} ({} chars)", session_id, content.len());

                // Forward to SessionHistory if this is the current session
                if let Some(current_id) = &self.current_session_id {
                    if current_id == session_id {
                        // Set thinking state
                        self.thinking_active = true;
                        self.update_chat_input_state(cx);

                        // Forward thinking delta to SessionHistory (tuple variant)
                        cx.action(HistoryAction::ThinkingDelta(content.clone()));
                    }
                }
            }
            WSEvent::MessageAdded { session_id, role, content, timestamp } => {
                log!("WebSocket event: MessageAdded for session {} (role: {})", session_id, role);

                // If this is an assistant message, thinking is complete
                if role == "assistant" {
                    if let Some(current_id) = &self.current_session_id {
                        if current_id == session_id {
                            self.thinking_active = false;
                            self.update_chat_input_state(cx);
                        }
                    }
                }

                // Forward to SessionHistory if this is the current session
                if let Some(current_id) = &self.current_session_id {
                    if current_id == session_id {
                        let message = Message {
                            role: role.clone(),
                            content: content.clone(),
                            timestamp: Some(timestamp.clone()),
                            thinking: false,
                        };

                        // Append message to history
                        cx.action(HistoryAction::MessageAppended(message));
                    }
                }
            }
        }
    }
}

impl AppMain for App {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        // Handle window resize events
        // TODO: WindowResized event doesn't exist in current Makepad API
        // Need to find alternative way to handle window resizing
        /*
        if let Event::WindowResized(new_size) = event {
            self.window_width = new_size.width;
            self.window_height = new_size.height;
            log!("Window resized to: {}x{}", new_size.width, new_size.height);

            // Update mobile layout when window size changes
            self.update_mobile_layout(cx);
        }
        */

        // Handle actions first
        if let Event::Actions(actions) = event {
            self.handle_actions(cx, actions);
        }

        // Poll for WebSocket events
        if let Some(ws_manager) = &self.ws_manager {
            if let Some(ws_message) = ws_manager.poll_event() {
                match ws_message {
                    WSMessage::Connected => {
                        log!("WebSocket connected (UI thread)");
                        cx.action(AppAction::WebSocket(WSAction::Connected));
                    }
                    WSMessage::Disconnected => {
                        log!("WebSocket disconnected (UI thread)");
                        cx.action(AppAction::WebSocket(WSAction::Disconnected));
                    }
                    WSMessage::Event(ws_event) => {
                        log!("WebSocket event received (UI thread): {:?}", ws_event);
                        cx.action(AppAction::WebSocket(WSAction::EventReceived(ws_event)));
                    }
                    WSMessage::Error(err) => {
                        log!("WebSocket error (UI thread): {}", err);
                        cx.action(AppAction::WebSocket(WSAction::Error(err)));
                    }
                }
            }
        }

        self.match_event(cx, event);
        let _ = self.ui.handle_event(cx, event, &mut Scope::empty());
    }
}

app_main!(App);

impl App {
    /// Check if we're in mobile mode (width < 768px)
    fn is_mobile(&self) -> bool {
        self.window_width < 768.0
    }

    /// Update sidebar visibility based on mobile mode and collapse state
    fn update_mobile_layout(&mut self, cx: &mut Cx) {
        let sidebar = self.ui.view(id!(sidebar));
        let main_panel = self.ui.view(id!(main_panel));

        if self.is_mobile() {
            // On mobile, sidebar is a full-width overlay
            if self.sidebar_collapsed {
                // Sidebar collapsed: show main panel, hide sidebar
                sidebar.set_visible(cx, false);
                main_panel.set_visible(cx, true);
            } else {
                // Sidebar expanded: show sidebar as overlay, hide main panel
                sidebar.set_visible(cx, true);
                main_panel.set_visible(cx, false);
            }
        } else {
            // On desktop, sidebar and main panel are side-by-side
            // Sidebar can be explicitly collapsed
            if self.sidebar_collapsed {
                sidebar.set_visible(cx, false);
            } else {
                sidebar.set_visible(cx, true);
            }
            // Main panel is always visible on desktop
            main_panel.set_visible(cx, true);
        }

        self.ui.redraw(cx);
    }

    // Fetch backend health check
    // This sets up the HTTP fetch infrastructure
    // Full async implementation will be added in future stories
    fn fetch_backend_health(&self, cx: &mut Cx) {
        let url = format!("{}/health", self.backend_url);
        log!("Fetching from: {}", url);

        // For WASM build - use browser fetch (to be implemented)
        #[cfg(target_arch = "wasm32")]
        {
            log!("WASM build detected - HTTP fetch infrastructure ready");
            self.ui.label(&[live_id!(status_label)])
                .set_text(cx, "WASM build - HTTP fetch ready for implementation");
        }

        // For desktop build - synchronous demo for now
        // Async implementation with proper spawn will be added in future stories
        #[cfg(not(target_arch = "wasm32"))]
        {
            log!("Desktop build detected - HTTP fetch infrastructure ready");
            // Infrastructure is set up:
            // - reqwest dependency added
            // - AppAction enum for async responses
            // - handle_actions for processing responses
            // Full async fetch will be implemented in future stories
            self.ui.label(&[live_id!(status_label)])
                .set_text(cx, "Desktop build - HTTP fetch ready");

            // Emit action to demonstrate the pattern
            cx.action(AppAction::HealthCheck("Infrastructure ready".to_string()));
        }
    }

    /// Update stop button visibility based on current session's active state
    fn update_stop_button_visibility(&mut self, cx: &mut Cx) {
        let stop_button = self.ui.button(id!(stop_session_button));

        // Only show stop button if we have a current session and it's active
        let should_show = self.current_session_id.is_some() && self.current_session_active;

        stop_button.set_visible(cx, should_show);
        self.ui.redraw(cx);

        log!("Stop button visibility: {} (session_active: {})",
              should_show, self.current_session_active);
    }

    /// Update chat input state based on current session's active state
    fn update_chat_input_state(&mut self, cx: &mut Cx) {
        // Enable chat input only if we have a current session, it's active, and not thinking
        let enabled = self.current_session_id.is_some()
            && self.current_session_active
            && !self.thinking_active;

        self.ui.text_input(id!(chat_input.message_input))
            .set_disabled(cx, !enabled);
        self.ui.button(id!(chat_input.send_button))
            .set_disabled(cx, !enabled);

        self.ui.redraw(cx);

        log!("Chat input state: {} (session_active: {}, thinking_active: {})",
              enabled, self.current_session_active, self.thinking_active);
    }
}
