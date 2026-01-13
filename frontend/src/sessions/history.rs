use makepad_widgets::*;
use crate::sessions::Message;
use crossbeam_channel::{Receiver, Sender, unbounded};

#[cfg(not(target_arch = "wasm32"))]
use tokio::spawn;

// Action for session history events
#[derive(Debug, Clone, DefaultNone)]
pub enum HistoryAction {
    None,
    MessagesLoaded(Vec<Message>),
    MessageAppended(Message),
    SessionChanged(String),
    ThinkingDelta(String),
    ThinkingAppended(String),
}

live_design! {
    import makepad_widgets::root::*;
    import makepad_widgets::theme::*;

    // User message bubble (right-aligned)
    UserMessageBubble = <View> {
        width: Fill
        height: Fit
        flow: Right
        margin: { top: 8, bottom: 8, left: 60, right: 8 }

        <View> {
            width: Fill
            height: Fit
            align: { x: 1.0, y: 0.0 }

            <View> {
                width: Fit
                max_width: 500
                height: Fit
                padding: 12
                flow: Down
                spacing: 8

                show_bg: true
                draw_bg: {
                    color: #3b82f6
                    instance border_radius: 12.0
                    fn pixel(self) -> vec4 {
                        let sdf = Sdf2d::viewport(self.pos * self.rect_size)
                        sdf.box(0., 0., self.rect_size.x, self.rect_size.y, self.border_radius)
                        sdf.fill(self.color)
                        return sdf.result
                    }
                }

                message_content = <Label> {
                    width: Fill
                    text: "User message"
                    draw_text: {
                        color: #ffffff
                        text_style: { font_size: 14.0, line_spacing: 1.4 }
                    }
                }
            }
        }
    }

    // Assistant message bubble (left-aligned)
    AssistantMessageBubble = <View> {
        width: Fill
        height: Fit
        flow: Right
        margin: { top: 8, bottom: 8, left: 8, right: 60 }

        <View> {
            width: Fill
            height: Fit
            align: { x: 0.0, y: 0.0 }

            <View> {
                width: Fit
                max_width: 500
                height: Fit
                padding: 12
                flow: Down
                spacing: 8

                show_bg: true
                draw_bg: {
                    color: #2a2a2a
                    instance border_radius: 12.0
                    fn pixel(self) -> vec4 {
                        let sdf = Sdf2d::viewport(self.pos * self.rect_size)
                        sdf.box(0., 0., self.rect_size.x, self.rect_size.y, self.border_radius)
                        sdf.fill(self.color)
                        return sdf.result
                    }
                }

                message_content = <Label> {
                    width: Fill
                    text: "Assistant message"
                    draw_text: {
                        color: #e0e0e0
                        text_style: { font_size: 14.0, line_spacing: 1.4 }
                    }
                }
            }
        }
    }

    // Code block display
    CodeBlock = <View> {
        width: Fill
        height: Fit
        margin: { top: 4 }

        <View> {
            width: Fill
            height: Fit
            padding: 12
            flow: Down
            spacing: 8

            show_bg: true
            draw_bg: {
                color: #1e1e1e
                instance border_radius: 6.0
                fn pixel(self) -> vec4 {
                    let sdf = Sdf2d::viewport(self.pos * self.rect_size)
                    sdf.box(0., 0., self.rect_size.x, self.rect_size.y, self.border_radius)
                    sdf.fill(self.color)
                    return sdf.result
                }
            }

            <View> {
                width: Fill
                height: Fit
                flow: Right
                spacing: 8

                <Label> {
                    text: "Code"
                    draw_text: {
                        color: #888
                        text_style: { font_size: 11.0, font_weight: 600.0 }
                    }
                }
            }

            code_content = <Label> {
                width: Fill
                text: "code here"
                draw_text: {
                    color: #d4d4d4
                    text_style: {
                        font_size: 13.0,
                        font_family: "monospace",
                        line_spacing: 1.4
                    }
                }
            }
        }
    }

    // Thinking block component for displaying assistant's thinking process
    ThinkingBlock = <View> {
        width: Fill
        height: Fit
        margin: { top: 8, bottom: 8, left: 60, right: 60 }

        <View> {
            width: Fill
            height: Fit
            flow: Down

            <View> {
                width: Fill
                height: Fit
                flow: Right
                padding: { top: 8, bottom: 8, left: 12, right: 12 }
                spacing: 8
                align: { y: 0.5 }

                show_bg: true
                draw_bg: {
                    color: #2a2a2a
                    instance border_radius: 8.0
                    fn pixel(self) -> vec4 {
                        let sdf = Sdf2d::viewport(self.pos * self.rect_size)
                        sdf.box(0., 0., self.rect_size.x, self.rect_size.y, self.border_radius)
                        sdf.fill(self.color)
                        return sdf.result
                    }
                }

                toggle_button = <Button> {
                    width: 24
                    height: 24
                    text: "▼"
                    draw_text: {
                        color: #888
                        text_style: { font_size: 14.0 }
                    }
                    draw_bg: {
                        color: transparent
                        color_hover: #333
                        border_radius: 4.0
                    }
                }

                <Label> {
                    text: "Thinking"
                    draw_text: {
                        color: #aaa
                        text_style: { font_size: 12.0, font_weight: 600.0 }
                    }
                }

                thinking_spinner = <Label> {
                    text: ""
                    draw_text: {
                        color: #3b82f6
                        text_style: { font_size: 12.0 }
                    }
                }
            }

            thinking_content = <View> {
                width: Fill
                height: Fit
                padding: { left: 12, right: 12, bottom: 12 }

                thinking_text = <Label> {
                    width: Fill
                    text: ""
                    draw_text: {
                        color: #e0e0e0
                        text_style: {
                            font_size: 13.0,
                            line_spacing: 1.4,
                            font_style: "italic"
                        }
                    }
                }
            }
        }
    }

    // Message item template
    MessageItem = <View> {
        width: Fill
        height: Fit
        flow: Down
        spacing: 4

        user_bubble = <UserMessageBubble> {
            visible: false
        }

        assistant_bubble = <AssistantMessageBubble> {
            visible: false
        }

        // Code block (optional)
        code_block = <CodeBlock> {
            visible: false
        }

        // Timestamp
        timestamp_label = <Label> {
            width: Fill
            text: ""
            margin: { top: 4, left: 8 }
            draw_text: {
                color: #666
                text_style: { font_size: 11.0 }
            }
        }
    }

    // Session history view
    SessionHistory = {{SessionHistory}} {
        width: Fill
        height: Fill
        flow: Down

        <View> {
            width: Fill
            height: Fill
            flow: Down
            padding: 16

            header = <View> {
                width: Fill
                height: Fit
                padding: { bottom: 16 }

                session_title = <Label> {
                    text: "Session History"
                    draw_text: {
                        color: #e0e0e0
                        text_style: { font_size: 24.0, font_weight: 700.0 }
                    }
                }
            }

            messages_container = <ScrollYView> {
                width: Fill
                height: Fill
                flow: Down

                <View> {
                    width: Fill
                    height: Fit
                    flow: Down
                    spacing: 8

                    empty_state = <View> {
                        width: Fill
                        height: Fill
                        align: { x: 0.5, y: 0.5 }
                        padding: 40

                        <Label> {
                            text: "Select a session to view its history"
                            draw_text: {
                                color: #666
                                text_style: { font_size: 14.0 }
                            }
                        }
                    }

                    // Thinking block
                    thinking_block = <ThinkingBlock> {
                        visible: false
                    }

                    // Message list (dynamically populated)
                    messages_list = <View> {
                        width: Fill
                        height: Fit
                        flow: Down
                        visible: false
                    }
                }
            }

            // Template for creating message items dynamically
            message_template: <MessageItem> {}
        }
    }
}

#[derive(Live, LiveHook)]
pub struct SessionHistory {
    #[live]
    ui: WidgetRef,

    #[live]
    message_template: Option<LivePtr>,

    #[rust]
    messages: Vec<Message>,

    #[rust]
    message_widgets: Vec<WidgetRef>,

    #[rust]
    current_session_id: Option<String>,

    #[rust]
    backend_url: String,

    #[rust]
    thinking_content: String,

    #[rust]
    thinking_expanded: bool,

    #[rust]
    thinking_active: bool,

    #[rust]
    action_receiver: Option<Receiver<HistoryAction>>,

    #[rust]
    action_sender: Option<Sender<HistoryAction>>,
}

impl LiveRegister for SessionHistory {
    fn live_register(cx: &mut Cx) {
        makepad_widgets::live_design(cx);
        crate::sessions::history::live_design(cx);
    }
}

impl MatchEvent for SessionHistory {
    fn handle_startup(&mut self, _cx: &mut Cx) {
        // Initialize backend_url from environment if not set
        if self.backend_url.is_empty() {
            self.backend_url = std::env::var("BACKEND_URL")
                .unwrap_or_else(|_| "http://localhost:8765".to_string());
        }

        log!("SessionHistory initialized with backend: {}", self.backend_url);
    }

    fn handle_actions(&mut self, cx: &mut Cx, actions: &Actions) {
        // Handle thinking block toggle button
        if self.ui.button(id!(toggle_button)).clicked(&actions) {
            self.thinking_expanded = !self.thinking_expanded;

            // Update toggle button text
            let toggle_text = if self.thinking_expanded { "▼" } else { "▶" };
            self.thinking_block().button(id!(toggle_button))
                .set_text(cx, toggle_text);

            // Show/hide thinking content
            self.thinking_block().view(id!(thinking_content))
                .set_visible(cx, self.thinking_expanded);

            self.ui.redraw(cx);
        }

        // Handle history actions
        for action in actions {
            if let Some(history_action) = action.downcast_ref::<HistoryAction>() {
                match history_action {
                    HistoryAction::MessagesLoaded(messages) => {
                        self.messages = messages.clone();

                        // Clear thinking state when loading new messages
                        self.thinking_content.clear();
                        self.thinking_active = false;
                        self.update_thinking_block(cx);

                        self.render_messages(cx);
                    }
                    HistoryAction::MessageAppended(message) => {
                        // Append message to existing list
                        self.messages.push(message.clone());
                        self.render_messages(cx);
                    }
                    HistoryAction::SessionChanged(session_id) => {
                        self.current_session_id = Some(session_id.clone());

                        // Clear thinking state when changing session
                        self.thinking_content.clear();
                        self.thinking_active = false;
                        self.update_thinking_block(cx);

                        // Update the title
                        self.ui.label(id!(session_title))
                            .set_text(cx, &format!("Session: {}", session_id));

                        // Fetch messages
                        self.fetch_messages(cx, session_id);
                    }
                    HistoryAction::ThinkingDelta(delta) => {
                        // Append delta to thinking content
                        self.thinking_content.push_str(delta);
                        self.thinking_active = true;
                        self.update_thinking_block(cx);
                    }
                    HistoryAction::ThinkingAppended(content) => {
                        // Append content to thinking
                        self.thinking_content.push_str(content);
                        self.thinking_active = true;
                        self.update_thinking_block(cx);
                    }
                    HistoryAction::None => {}
                }
            }
        }
    }
}

impl SessionHistory {
    /// Set the current session and load its messages
    pub fn set_session(&mut self, cx: &mut Cx, session_id: &str, session_name: &str) {
        self.current_session_id = Some(session_id.to_string());

        // Update title
        self.ui.label(id!(session_title))
            .set_text(cx, &format!("Session: {}", session_name));

        // Fetch messages
        self.fetch_messages(cx, session_id);
    }

    /// Fetch messages for the current session from the backend
    fn fetch_messages(&self, cx: &mut Cx, session_id: &str) {
        let url = format!("{}/api/sessions/{}/messages", self.backend_url, session_id);
        log!("Fetching messages from: {}", url);

        #[cfg(not(target_arch = "wasm32"))]
        {
            let fetch_url = url.clone();
            let backend_url = self.backend_url.clone();

            spawn(async move {
                match reqwest::get(&fetch_url).await {
                    Ok(response) => {
                        match response.json::<Vec<Message>>().await {
                            Ok(messages) => {
                                log!("Successfully loaded {} messages", messages.len());
                                Cx::post_action(HistoryAction::MessagesLoaded(messages));
                            }
                            Err(e) => {
                                log!("Failed to parse messages JSON: {}", e);
                                // Use empty messages on error
                                Cx::post_action(HistoryAction::MessagesLoaded(vec![]));
                            }
                        }
                    }
                    Err(e) => {
                        log!("Failed to fetch messages: {}", e);
                        log!("Using demo data as fallback");

                        // Fallback to demo messages if backend is unavailable
                        let demo_messages = vec![
                            Message {
                                role: "user".to_string(),
                                content: "Help me refactor this code".to_string(),
                                timestamp: Some("2026-01-13T10:30:00Z".to_string()),
                                thinking: false,
                            },
                            Message {
                                role: "assistant".to_string(),
                                content: "I'll help you refactor the code. Let me first analyze the structure.".to_string(),
                                timestamp: Some("2026-01-13T10:30:05Z".to_string()),
                                thinking: false,
                            },
                            Message {
                                role: "assistant".to_string(),
                                content: "Here's a refactored version:\n\n```rust\nfn process_data(input: &str) -> Result<String> {\n    input\n        .lines()\n        .filter(|line| !line.is_empty())\n        .map(|line| line.trim())\n        .collect::<Vec<_>>()\n        .join(\"\\n\")\n        .pipe(|s| Ok(s))\n}\n```\n\nThis version is more functional and easier to test.".to_string(),
                                timestamp: Some("2026-01-13T10:30:15Z".to_string()),
                                thinking: false,
                            },
                            Message {
                                role: "user".to_string(),
                                content: "Thanks! Can you explain the pipe method?".to_string(),
                                timestamp: Some("2026-01-13T10:31:00Z".to_string()),
                                thinking: false,
                            },
                        ];
                        Cx::post_action(HistoryAction::MessagesLoaded(demo_messages));
                    }
                }
            });
        }

        #[cfg(target_arch = "wasm32")]
        {
            // WASM build - use browser fetch
            log!("WASM build - using browser fetch API");
            cx.action(HistoryAction::MessagesLoaded(vec![]));
        }
    }

    /// Render messages in the UI using template widgets
    fn render_messages(&mut self, cx: &mut Cx) {
        log!("Rendering {} messages", self.messages.len());

        let empty_state = self.ui.view(id!(empty_state));
        let messages_list = self.ui.view(id!(messages_list));

        if self.messages.is_empty() {
            empty_state.set_visible(cx, true);
            messages_list.set_visible(cx, false);

            self.ui.label(id!(session_title))
                .set_text(cx, "Session History");
        } else {
            empty_state.set_visible(cx, false);
            messages_list.set_visible(cx, true);

            // Create or update message widgets using template
            self.update_message_widgets(cx);
        }

        self.ui.redraw(cx);
    }

    /// Update message widgets using the template pattern
    fn update_message_widgets(&mut self, cx: &mut Cx) {
        // Ensure we have enough widgets for all messages
        while self.message_widgets.len() < self.messages.len() {
            let widget = WidgetRef::new_from_ptr(cx, self.message_template);
            self.message_widgets.push(widget);
        }

        // Hide excess widgets
        for (i, widget) in self.message_widgets.iter().enumerate() {
            if i >= self.messages.len() {
                widget.set_visible(cx, false);
            }
        }

        // Update each message widget
        for (i, message) in self.messages.iter().enumerate() {
            if let Some(widget) = self.message_widgets.get(i) {
                widget.set_visible(cx, true);

                let is_user = message.role == "user";

                // Show/hide appropriate bubble
                let user_bubble = widget.view(id!(user_bubble));
                let assistant_bubble = widget.view(id!(assistant_bubble));

                user_bubble.set_visible(cx, is_user);
                assistant_bubble.set_visible(cx, !is_user);

                // Format content without code blocks (they'll be shown separately)
                let content_text = self.extract_text_content(&message.content);

                // Set message content in the appropriate bubble
                if is_user {
                    // Access user_bubble's message_content label
                    // We need to navigate through the nested structure
                    widget.view(id!(user_bubble))
                        .label(id!(message_content))
                        .set_text(cx, &content_text);
                } else {
                    widget.view(id!(assistant_bubble))
                        .label(id!(message_content))
                        .set_text(cx, &content_text);
                }

                // Handle code blocks
                let code_blocks = self.extract_code_blocks(&message.content);
                let code_block = widget.view(id!(code_block));

                if !code_blocks.is_empty() {
                    code_block.set_visible(cx, true);

                    // Combine all code blocks
                    let combined_code = code_blocks.join("\n");
                    code_block.label(id!(code_content))
                        .set_text(cx, &combined_code);
                } else {
                    code_block.set_visible(cx, false);
                }

                // Set timestamp
                let timestamp_text = message.timestamp.as_ref()
                    .map(|ts| self.format_timestamp(ts))
                    .unwrap_or_else(|| "".to_string());

                widget.label(id!(timestamp_label))
                    .set_text(cx, &timestamp_text);
            }
        }
    }

    /// Extract text content without code blocks
    fn extract_text_content(&self, content: &str) -> String {
        let mut result = String::new();
        let mut in_code_block = false;

        for line in content.lines() {
            if line.trim().starts_with("```") {
                in_code_block = !in_code_block;
            } else if !in_code_block {
                if !result.is_empty() {
                    result.push('\n');
                }
                result.push_str(line);
            }
        }

        result
    }

    /// Format timestamp for display
    fn format_timestamp(&self, timestamp: &str) -> String {
        // Simple formatting - just return the timestamp as-is for now
        // In a full implementation, would parse and format relative time
        timestamp.to_string()
    }

    /// Extract code from markdown code blocks
    fn extract_code_blocks(&self, content: &str) -> Vec<String> {
        let mut blocks = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut in_code_block = false;
        let mut current_block = String::new();

        for line in lines {
            if line.trim().starts_with("```") {
                if in_code_block {
                    // End of code block
                    if !current_block.is_empty() {
                        blocks.push(current_block.clone());
                    }
                    current_block.clear();
                    in_code_block = false;
                } else {
                    // Start of code block
                    in_code_block = true;
                }
            } else if in_code_block {
                current_block.push_str(line);
                current_block.push('\n');
            }
        }

        blocks
    }

    /// Set backend URL
    pub fn set_backend_url(&mut self, url: String) {
        self.backend_url = url;
    }

    /// Get a reference to the thinking block view
    fn thinking_block(&self) -> ViewRef {
        self.ui.view(id!(thinking_block))
    }

    /// Update the thinking block UI based on current state
    fn update_thinking_block(&mut self, cx: &mut Cx) {
        let thinking_block = self.thinking_block();
        let has_content = !self.thinking_content.is_empty();

        // Show thinking block if there's content or thinking is active
        thinking_block.set_visible(cx, has_content || self.thinking_active);

        if has_content || self.thinking_active {
            // Update thinking text
            thinking_block.label(id!(thinking_text))
                .set_text(cx, &self.thinking_content);

            // Update spinner text - show "..." when thinking but no content yet
            let spinner_text = if has_content { "" } else { "..." };
            thinking_block.label(id!(thinking_spinner))
                .set_text(cx, spinner_text);

            // Ensure thinking content is visible if expanded
            thinking_block.view(id!(thinking_content))
                .set_visible(cx, self.thinking_expanded && has_content);
        }

        self.ui.redraw(cx);
    }
}
