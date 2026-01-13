use makepad_widgets::*;
use makepad_widgets::event::KeyEvent;

#[cfg(not(target_arch = "wasm32"))]
use tokio::spawn;

// Action for chat input events
#[derive(Debug, Clone, DefaultNone)]
pub enum ChatInputAction {
    None,
    MessageSent { content: String },
    SetBusy { busy: bool },
    SetSession { session_id: String },
}

live_design! {
    import makepad_widgets::root::*;
    import makepad_widgets::theme::*;

    // Chat input component with text area and send button
    ChatInput = {{ChatInput}} {
        width: Fill
        height: Fit
        flow: Down

        <View> {
            width: Fill
            height: Fit
            flow: Right
            padding: 12
            spacing: 8

            show_bg: true
            draw_bg: {
                color: #242424
                border_radius: 8.0
            }

            // Text input for message
            message_input = <TextInput> {
                width: Fill
                height: Fill
                text: ""

                draw_bg: {
                    color: #1a1a1a
                    color_focus: #2a2a2a
                    border_radius: 6.0
                }
                draw_text: {
                    color: #e0e0e0
                    text_style: { font_size: 16.0, line_spacing: 1.4 }
                }

                // Enable multiline input
                draw_bg: {
                    fn pixel(self) -> vec4 {
                        let sdf = Sdf2d::viewport(self.pos * self.rect_size)
                        sdf.box(0., 0., self.rect_size.x, self.rect_size.y, self.border_radius)
                        sdf.fill(self.color)
                        return sdf.result
                    }
                }
            }

            // Send button (touch-friendly: 44px minimum)
            send_button = <Button> {
                width: Fit
                height: 44
                min_width: 80
                text: "Send"

                draw_text: {
                    color: #ffffff
                    text_style: { font_size: 16.0, font_weight: 600.0 }
                }
                draw_bg: {
                    color: #3b82f6
                    color_hover: #2b6fd6
                    color_disabled: #444
                    border_radius: 6.0
                    fn pixel(self) -> vec4 {
                        let sdf = Sdf2d::viewport(self.pos * self.rect_size)
                        sdf.box(0., 0., self.rect_size.x, self.rect_size.y, self.border_radius)
                        sdf.fill(self.color)
                        return sdf.result
                    }
                }
            }
        }

        // Hint text for shortcuts
        <View> {
            width: Fill
            height: Fit
            padding: { left: 12, right: 12, bottom: 8 }

            <Label> {
                text: "Press Enter to send, Shift+Enter for new line"
                draw_text: {
                    color: #666
                    text_style: { font_size: 13.0 }
                }
            }
        }
    }
}

#[derive(Live, LiveHook, Default)]
pub struct ChatInput {
    #[live]
    ui: WidgetRef,

    #[rust]
    backend_url: String,

    #[rust]
    current_session_id: Option<String>,

    #[rust]
    is_busy: bool,
}

impl LiveRegister for ChatInput {
    fn live_register(cx: &mut Cx) {
        makepad_widgets::live_design(cx);
        crate::sessions::chat_input::live_design(cx);
    }
}

impl MatchEvent for ChatInput {
    fn handle_startup(&mut self, _cx: &mut Cx) {
        // Initialize backend_url from environment if not set
        if self.backend_url.is_empty() {
            self.backend_url = std::env::var("BACKEND_URL")
                .unwrap_or_else(|_| "http://localhost:8765".to_string());
        }

        log!("ChatInput initialized with backend: {}", self.backend_url);
    }

    fn handle_actions(&mut self, cx: &mut Cx, actions: &Actions) {
        // Handle send button click
        if self.ui.button(id!(send_button)).clicked(&actions) {
            self.send_message(cx);
        }

        // Handle text input - check for Enter key
        let text_input = self.ui.text_input(id!(message_input));
        if text_input.changed(actions).is_some() {
            // Text changed - could be used for validation
        }

        // Check for Enter key press
        for action in actions {
            if let Some(key_event) = action.downcast_ref::<KeyEvent>() {
                // Check if this is the Enter key
                let is_enter = format!("{:?}", key_event.key_code).contains("Enter");

                if is_enter {
                    // Check if Shift is pressed
                    let shift_pressed = key_event.modifiers.shift;

                    if shift_pressed {
                        // Shift+Enter: Insert new line (default behavior)
                        // Don't need to do anything special
                    } else {
                        // Enter without Shift: Send message
                        self.send_message(cx);
                    }
                }
            }
        }

        // Handle ChatInputAction events
        for action in actions {
            if let Some(chat_action) = action.downcast_ref::<ChatInputAction>() {
                match chat_action {
                    ChatInputAction::SetBusy { busy } => {
                        self.set_busy(cx, *busy);
                    }
                    ChatInputAction::SetSession { session_id } => {
                        self.set_session(cx, session_id);
                    }
                    ChatInputAction::MessageSent { .. } => {
                        // Already handled above via send button/Enter key
                    }
                    ChatInputAction::None => {}
                }
            }
        }
    }
}

impl ChatInput {
    /// Set the current session ID
    pub fn set_session(&mut self, _cx: &mut Cx, session_id: &str) {
        self.current_session_id = Some(session_id.to_string());
    }

    /// Set the busy state (disable input when thinking)
    pub fn set_busy(&mut self, cx: &mut Cx, busy: bool) {
        self.is_busy = busy;

        // Enable/disable the input and button
        self.ui.text_input(id!(message_input))
            .set_disabled(cx, busy);
        self.ui.button(id!(send_button))
            .set_disabled(cx, busy);

        self.ui.redraw(cx);
    }

    /// Send the current message
    fn send_message(&mut self, cx: &mut Cx) {
        // Don't send if busy
        if self.is_busy {
            log!("ChatInput is busy, ignoring send request");
            return;
        }

        // Don't send if no session is selected
        if self.current_session_id.is_none() {
            log!("No session selected, ignoring send request");
            return;
        }

        // Get the message content
        let message_input = self.ui.text_input(id!(message_input));
        let content = message_input.text();

        let trimmed = content.trim();

        if trimmed.is_empty() {
            log!("Empty message, ignoring send request");
            return;
        }

        log!("Sending message: {}", trimmed);

        // Send the message to the backend
        self.send_to_backend(cx, trimmed);

        // Emit action to notify parent (for optimistic update)
        cx.action(ChatInputAction::MessageSent {
            content: trimmed.to_string(),
        });

        // Clear the input
        message_input.set_text(cx, "");

        self.ui.redraw(cx);
    }

    /// Send the message to the backend via POST /api/sessions/:id/prompt
    fn send_to_backend(&self, cx: &mut Cx, content: &str) {
        let session_id = match &self.current_session_id {
            Some(id) => id.clone(),
            None => {
                log!("No session selected, not sending to backend");
                return;
            }
        };

        let url = format!("{}/api/sessions/{}/prompt", self.backend_url, session_id);
        log!("Sending prompt to: {}", url);

        #[cfg(not(target_arch = "wasm32"))]
        {
            let fetch_url = url.clone();
            let prompt = content.to_string();

            spawn(async move {
                match reqwest::Client::new()
                    .post(&fetch_url)
                    .json(&serde_json::json!({ "prompt": prompt }))
                    .send()
                    .await
                {
                    Ok(response) => {
                        if response.status().is_success() {
                            log!("Prompt sent successfully");
                        } else {
                            let status = response.status();
                            log!("Failed to send prompt, status: {}", status);
                        }
                    }
                    Err(e) => {
                        log!("Failed to send prompt: {}", e);
                    }
                }
            });
        }

        #[cfg(target_arch = "wasm32")]
        {
            log!("WASM build - prompt sending not yet implemented");
            // Browser fetch would use web-sys or wasm-bindgen fetch
            // For now, this is a placeholder
        }
    }

    /// Set backend URL
    pub fn set_backend_url(&mut self, url: String) {
        self.backend_url = url;
    }
}
