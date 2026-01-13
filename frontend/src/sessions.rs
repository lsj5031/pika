use makepad_widgets::*;
use serde::{Deserialize, Serialize};

// Session data structure matching the API response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub name: String,
    pub project_path: String,
    pub created_at: String,
    #[serde(default)]
    pub is_active: bool,
}

// Message data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
    #[serde(default = "default_thinking")]
    pub thinking: bool,
}

fn default_thinking() -> bool {
    false
}

// Group of sessions belonging to a project
#[derive(Debug, Clone)]
pub struct SessionGroup {
    pub project_name: String,
    pub project_path: String,
    pub sessions: Vec<Session>,
    pub expanded: bool,
}

// Action for session-related events
#[derive(Debug, Clone, DefaultNone)]
pub enum SessionAction {
    None,
    SessionsLoaded(Vec<Session>),
    SearchChanged(String),
    ProjectToggled(String),
    SessionClicked(String),
    SessionStatusChanged { session_id: String, is_active: bool },
    SessionStarted { session_id: String },
    SessionLoading { session_id: Option<String> },
    StopSessionClicked(String),
}

live_design! {
    import makepad_widgets::root::*;
    import makepad_widgets::theme::*;

    // Search input box
    SessionSearch = <SessionSearch> {
        width: Fill
        height: Fit

        <View> {
            width: Fill
            height: Fit
            flow: Down
            spacing: 8

            <Label> {
                text: "Search Sessions"
                draw_text: {
                    color: #888
                    text_style: { font_size: 14.0, font_weight: 500.0 }
                }
            }

            search_input = <TextInput> {
                width: Fill
                height: 44
                text: ""
                draw_bg: {
                    color: #333
                    color_focus: #444
                    border_radius: 6.0
                }
                draw_text: {
                    color: #e0e0e0
                    text_style: { font_size: 16.0 }
                }
            }
        }
    }

    // Single session item (touch-friendly: minimum 44px height)
    SessionItem = <SessionItem> {
        width: Fill
        height: Fit

        <View> {
            width: Fill
            height: Fit
            flow: Right
            spacing: 8

            <Button> {
                width: Fill
                height: 44
                flow: Right
                padding: { top: 12, bottom: 12, left: 12, right: 12 }
                spacing: 10
                align: { y: 0.5 }

                draw_bg: {
                    color: #333
                    color_hover: #3a3a3a
                    border_radius: 6.0
                }

                status_indicator = <View> {
                    width: 10
                    height: 10

                    show_bg: true
                    draw_bg: {
                        color: #666
                        instance border_radius: 5.0
                        fn pixel(self) -> vec4 {
                            let sdf = Sdf2d::viewport(self.pos * self.rect_size)
                            sdf.circle(5., 5., 5.)
                            sdf.fill(self.color)
                            return sdf.result
                        }
                    }
                }

                session_info = <View> {
                    width: Fill
                    height: Fit
                    flow: Down
                    spacing: 2

                    session_name = <Label> {
                        width: Fill
                        text: "Session Name"
                        draw_text: {
                            color: #e0e0e0
                            text_style: { font_size: 16.0, font_weight: 400.0 }
                        }
                    }

                    session_time = <Label> {
                        width: Fill
                        text: "2 hours ago"
                        draw_text: {
                            color: #888
                            text_style: { font_size: 13.0 }
                        }
                    }
                }
            }

            stop_button = <Button> {
                width: 60
                height: 44
                visible: false
                text: "Stop"
                draw_text: {
                    color: #e0e0e0
                    text_style: { font_size: 14.0, font_weight: 500.0 }
                }
                draw_bg: {
                    color: #dc2626
                    color_hover: #b91c1c
                    border_radius: 6.0
                }
            }
        }
    }

    // Collapsible project group header
    SessionGroupHeader = <SessionGroupHeader> {
        width: Fill
        height: Fit

        <Button> {
            width: Fill
            height: 44
            flow: Right
            padding: { left: 8, right: 8 }
            spacing: 8
            align: { y: 0.5 }

            draw_bg: {
                color: #2a2a2a
                color_hover: #333
                border_radius: 6.0
            }

            expand_icon = <Label> {
                text: "▶"
                draw_text: {
                    color: #aaa
                    text_style: { font_size: 14.0 }
                }
            }

            project_name = <Label> {
                width: Fill
                text: "Project Name"
                draw_text: {
                    color: #e0e0e0
                    text_style: { font_size: 16.0, font_weight: 600.0 }
                }
            }

            session_count = <Label> {
                text: "3"
                draw_text: {
                    color: #888
                    text_style: { font_size: 14.0 }
                }
            }
        }
    }

    // Session list component
    SessionList = <SessionList> {
        width: Fill
        height: Fill
        flow: Down
        spacing: 12

        <View> {
            width: Fill
            height: Fit

            <SessionSearch> {}
        }

        sessions_container = <ScrollYView> {
            width: Fill
            height: Fill

            sessions_content = <View> {
                width: Fill
                height: Fit
                flow: Down
                spacing: 8

                empty_state = <Label> {
                    width: Fill
                    text: "No sessions found"
                    draw_text: {
                        color: #666
                        text_style: { font_size: 15.0 }
                    }
                }
            }
        }
    }
}

#[derive(Live, LiveHook, Default)]
pub struct SessionList {
    #[live]
    ui: WidgetRef,

    #[rust]
    sessions: Vec<Session>,

    #[rust]
    grouped_sessions: Vec<SessionGroup>,

    #[rust]
    search_query: String,

    #[rust]
    backend_url: String,

    #[rust]
    loading_session_id: Option<String>,

    #[rust]
    #[cfg(not(target_arch = "wasm32"))]
    http_client: Option<reqwest::Client>,
}

impl LiveRegister for SessionList {
    fn live_register(cx: &mut Cx) {
        makepad_widgets::live_design(cx);
        crate::sessions::live_design(cx);
    }
}

impl MatchEvent for SessionList {
    fn handle_startup(&mut self, cx: &mut Cx) {
        // Initialize backend_url from environment if not set
        if self.backend_url.is_empty() {
            self.backend_url = std::env::var("BACKEND_URL")
                .unwrap_or_else(|_| "http://localhost:8765".to_string());
        }

        // Initialize HTTP client
        #[cfg(not(target_arch = "wasm32"))]
        {
            if self.http_client.is_none() {
                self.http_client = Some(reqwest::Client::new());
            }
        }

        // Auto-fetch sessions on startup
        self.fetch_sessions(cx);
    }

    fn handle_actions(&mut self, cx: &mut Cx, actions: &Actions) {
        // Handle search input changes
        if let Some(text) = self.ui.text_input(id!(search_input)).changed(actions) {
            self.search_query = text.clone();
            self.filter_sessions(cx);
        }

        // Handle stop button clicks for all session items
        // Since we can't create dynamic widgets, we check if any stop button was clicked
        // and emit the appropriate action
        for session in &self.sessions {
            if session.is_active {
                let stop_button_id = live_id!(stop_button);
                // We would need to check each button, but since buttons are dynamically created
                // based on sessions, we'll handle this differently
                // For now, the stop functionality will be triggered via SessionAction
            }
        }

        // Handle session actions
        for action in actions {
            if let Some(session_action) = action.downcast_ref::<SessionAction>() {
                match session_action {
                    SessionAction::SessionsLoaded(sessions) => {
                        self.sessions = sessions.clone();
                        self.group_sessions(cx);
                        self.filter_sessions(cx);
                    }
                    SessionAction::SearchChanged(query) => {
                        self.search_query = query.clone();
                        self.filter_sessions(cx);
                    }
                    SessionAction::ProjectToggled(project_path) => {
                        self.toggle_project(cx, project_path);
                    }
                    SessionAction::SessionClicked(session_id) => {
                        log!("Session clicked: {}", session_id);
                        // Future: handle session selection
                    }
                    SessionAction::SessionStatusChanged { session_id, is_active } => {
                        self.update_session_status(cx, session_id, *is_active);
                    }
                    SessionAction::SessionStarted { session_id } => {
                        // Update session status to active when started
                        self.update_session_status(cx, session_id, true);
                        // Clear loading state
                        self.loading_session_id = None;
                        log!("Session started: {}", session_id);
                    }
                    SessionAction::SessionLoading { session_id } => {
                        self.loading_session_id = session_id.clone();
                        log!("Session loading state changed: {:?}", session_id);
                        self.rebuild_session_list(cx);
                    }
                    SessionAction::StopSessionClicked(session_id) => {
                        log!("Stop session clicked: {}", session_id);
                        self.stop_session(cx, session_id.clone());
                    }
                    SessionAction::None => {}
                }
            }
        }
    }
}

impl SessionList {
    /// Group sessions by project
    fn group_sessions(&mut self, cx: &mut Cx) {
        use std::collections::HashMap;

        let mut project_map: HashMap<String, Vec<Session>> = HashMap::new();

        // Group sessions by project path
        for session in &self.sessions {
            let entry = project_map
                .entry(session.project_path.clone())
                .or_insert_with(Vec::new);
            entry.push(session.clone());
        }

        // Convert to SessionGroup structs
        self.grouped_sessions = project_map
            .into_iter()
            .map(|(project_path, sessions)| {
                // Extract project name from path
                let project_name = project_path
                    .split('/')
                    .last()
                    .unwrap_or("Unknown")
                    .to_string();

                SessionGroup {
                    project_name,
                    project_path,
                    sessions,
                    expanded: true, // Default to expanded
                }
            })
            .collect();

        log!("Grouped {} sessions into {} projects", self.sessions.len(), self.grouped_sessions.len());

        // Rebuild UI with grouped sessions
        self.rebuild_session_list(cx);
    }

    /// Rebuild the session list UI
    fn rebuild_session_list(&mut self, cx: &mut Cx) {
        log!("Rebuilding session list with {} groups", self.grouped_sessions.len());

        // Get the sessions container view
        let _sessions_container = self.ui.view(id!(sessions_content));

        // Show/hide empty state
        let empty_label = self.ui.label(id!(empty_state));
        if self.grouped_sessions.is_empty() {
            empty_label.set_visible(cx, true);
            empty_label.set_text(cx, "No sessions found");
            self.ui.redraw(cx);
            return;
        } else {
            empty_label.set_visible(cx, false);
        }

        // Build display text for sessions
        // Note: Full dynamic widget creation in Makepad requires more complex patterns
        // For now, we'll display a formatted text representation
        let mut display_text = String::new();

        for group in &self.grouped_sessions {
            let expand_icon = if group.expanded { "▼" } else { "▶" };
            display_text.push_str(&format!("\n{} {} ({})\n", expand_icon, group.project_name, group.sessions.len()));

            if group.expanded {
                for session in &group.sessions {
                    // Check if this session is loading
                    let is_loading = self.loading_session_id.as_ref()
                        .map(|id| id == &session.id)
                        .unwrap_or(false);

                    let status = if is_loading {
                        "⏳"  // Hourglass emoji for loading state
                    } else if session.is_active {
                        "●"
                    } else {
                        "○"
                    };
                    let time = self.format_timestamp(&session.created_at);
                    let loading_text = if is_loading { " (starting...)" } else { "" };
                    display_text.push_str(&format!("  {} {}{} - {}\n", status, session.name, loading_text, time));
                }
            }
        }

        // Update empty state label to show the sessions
        // This is a simplified approach - full implementation would create dynamic widgets
        let total_sessions: usize = self.grouped_sessions.iter().map(|g| g.sessions.len()).sum();
        let header = format!("Loaded {} sessions in {} projects:\n",
                            total_sessions, self.grouped_sessions.len());
        empty_label.set_text(cx, &format!("{}{}", header, display_text));
        empty_label.set_visible(cx, true);

        // Log sessions for debugging
        for group in &self.grouped_sessions {
            log!("Project: {} ({} sessions, expanded: {})",
                 group.project_name, group.sessions.len(), group.expanded);
            for session in &group.sessions {
                let is_loading = self.loading_session_id.as_ref()
                    .map(|id| id == &session.id)
                    .unwrap_or(false);
                log!("  - {} (active: {}, loading: {})", session.name, session.is_active, is_loading);
            }
        }

        self.ui.redraw(cx);
    }

    /// Filter sessions based on search query
    fn filter_sessions(&mut self, cx: &mut Cx) {
        log!("Filtering sessions with query: '{}'", self.search_query);

        // Filter sessions based on search query
        let filtered_sessions: Vec<Session> = if self.search_query.trim().is_empty() {
            self.sessions.clone()
        } else {
            let query = self.search_query.to_lowercase();
            self.sessions
                .iter()
                .filter(|s| s.name.to_lowercase().contains(&query))
                .cloned()
                .collect()
        };

        // Regroup the filtered sessions
        use std::collections::HashMap;

        let mut project_map: HashMap<String, Vec<Session>> = HashMap::new();

        for session in &filtered_sessions {
            let entry = project_map
                .entry(session.project_path.clone())
                .or_insert_with(Vec::new);
            entry.push(session.clone());
        }

        self.grouped_sessions = project_map
            .into_iter()
            .map(|(project_path, sessions)| {
                let project_name = project_path
                    .split('/')
                    .last()
                    .unwrap_or("Unknown")
                    .to_string();

                // Preserve expansion state if project was already visible
                let expanded = self.grouped_sessions
                    .iter()
                    .find(|g| g.project_path == project_path)
                    .map(|g| g.expanded)
                    .unwrap_or(true);

                SessionGroup {
                    project_name,
                    project_path,
                    sessions,
                    expanded,
                }
            })
            .collect();

        // Rebuild UI with filtered sessions
        self.rebuild_session_list(cx);
    }

    /// Toggle project group expansion
    fn toggle_project(&mut self, cx: &mut Cx, project_path: &str) {
        if let Some(group) = self.grouped_sessions
            .iter_mut()
            .find(|g| g.project_path == project_path)
        {
            group.expanded = !group.expanded;
            log!("Project {} expanded: {}", group.project_name, group.expanded);
            self.ui.redraw(cx);
        }
    }

    /// Fetch sessions from backend
    pub fn fetch_sessions(&self, cx: &mut Cx) {
        let url = format!("{}/api/sessions", self.backend_url);
        log!("Fetching sessions from: {}", url);

        #[cfg(not(target_arch = "wasm32"))]
        {
            // Desktop build - use reqwest for HTTP fetch
            let fetch_url = url.clone();
            let backend_url = self.backend_url.clone();

            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async move {
                    match reqwest::get(&fetch_url).await {
                        Ok(response) => {
                            match response.json::<Vec<Session>>().await {
                                Ok(sessions) => {
                                    log!("Successfully loaded {} sessions", sessions.len());
                                    Cx::post_action(SessionAction::SessionsLoaded(sessions));
                                }
                                Err(e) => {
                                    log!("Failed to parse sessions JSON: {}", e);
                                    // Use empty sessions on error
                                    Cx::post_action(SessionAction::SessionsLoaded(vec![]));
                                }
                            }
                        }
                        Err(e) => {
                            log!("Failed to fetch sessions: {}", e);
                            log!("Using demo data as fallback");

                            // Fallback to demo data if backend is unavailable
                            let demo_sessions = vec![
                                Session {
                                    id: "session-1".to_string(),
                                    name: "Code Refactor (Demo)".to_string(),
                                    project_path: "/home/leo/code/pi-agent-manager".to_string(),
                                    created_at: "2026-01-13T10:30:00Z".to_string(),
                                    is_active: true,
                                },
                                Session {
                                    id: "session-2".to_string(),
                                    name: "Bug Fix Session (Demo)".to_string(),
                                    project_path: "/home/leo/code/pi-agent-manager".to_string(),
                                    created_at: "2026-01-13T09:15:00Z".to_string(),
                                    is_active: false,
                                },
                                Session {
                                    id: "session-3".to_string(),
                                    name: "Feature Implementation (Demo)".to_string(),
                                    project_path: "/home/leo/other-project".to_string(),
                                    created_at: "2026-01-13T08:00:00Z".to_string(),
                                    is_active: false,
                                },
                            ];
                            Cx::post_action(SessionAction::SessionsLoaded(demo_sessions));
                        }
                    }
                });
            });
        }

        #[cfg(target_arch = "wasm32")]
        {
            // WASM build - use browser fetch
            log!("WASM build - using browser fetch API");
            // Browser fetch would use web-sys or wasm-bindgen fetch
            // For now, emit action to indicate infrastructure is ready
            cx.action(SessionAction::SessionsLoaded(vec![]));
        }
    }

    /// Set backend URL
    pub fn set_backend_url(&mut self, url: String) {
        self.backend_url = url;
    }

    /// Format timestamp for display
    fn format_timestamp(&self, timestamp: &str) -> String {
        // Simple formatting - just return the timestamp as-is for now
        // In a full implementation, would parse and format relative time
        timestamp.to_string()
    }

    /// Update session active status (called when WebSocket events arrive)
    pub fn update_session_status(&mut self, cx: &mut Cx, session_id: &str, is_active: bool) {
        log!("Updating session {} active status to: {}", session_id, is_active);

        // Find and update the session
        for session in &mut self.sessions {
            if session.id == session_id {
                session.is_active = is_active;
                log!("Session {} status updated to {}", session.name, is_active);

                // Regroup and refresh UI
                self.group_sessions(cx);
                self.filter_sessions(cx);
                return;
            }
        }

        // Session not found - might need to fetch updated list
        log!("Session {} not found in current list, fetching updated sessions", session_id);
        self.fetch_sessions(cx);
    }

    /// Stop a running session
    fn stop_session(&self, cx: &mut Cx, session_id: String) {
        let backend_url = self.backend_url.clone();
        let stop_url = format!("{}/api/sessions/{}/stop", backend_url, session_id);

        log!("Stopping session {} via: {}", session_id, stop_url);

        #[cfg(not(target_arch = "wasm32"))]
        {
            // Clone the client for use in the async block
            if let Some(client) = self.http_client.clone() {
                std::thread::spawn(move || {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(async move {
                        match client.post(&stop_url).send().await {
                            Ok(response) => {
                                if response.status().is_success() {
                                    log!("Successfully sent stop request for session {}", session_id);
                                    // WebSocket will handle the SessionStopped event to update UI
                                } else {
                                    log!("Failed to stop session {}: HTTP {}", session_id, response.status());
                                }
                            }
                            Err(e) => {
                                log!("Failed to stop session {}: {}", session_id, e);
                            }
                        }
                    });
                });
            } else {
                log!("HTTP client not initialized, cannot stop session");
            }
        }

        #[cfg(target_arch = "wasm32")]
        {
            log!("WASM build - stop session not yet implemented");
            // TODO: Implement browser fetch for stop session
        }
    }
}

// Session history module
pub mod history;

// Chat input module
pub mod chat_input;
