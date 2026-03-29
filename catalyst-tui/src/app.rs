use catalyst_core::AgentEvent;
use std::collections::HashMap;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputMode {
    Normal,
    Insert,
    ProviderSelect,
    ApiKeyInput,
}

#[derive(Debug, Clone)]
pub enum PopupState {
    None,
    ProviderSelect {
        selected: usize,
    },
    ApiKeyInput {
        provider_id: String,
        api_key_input: String,
    },
}

#[derive(Debug, Clone)]
pub enum Message {
    User {
        content: String,
    },
    Assistant {
        content: String,
        thinking: Option<String>,
    },
    ToolCall {
        id: String,
        name: String,
        status: ToolStatus,
    },
    ToolResult {
        id: String,
        output: String,
        is_error: bool,
    },
    System {
        content: String,
        level: SystemLevel,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SystemLevel {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ToolStatus {
    Pending,
    Running,
    Complete,
    Failed,
}

pub struct App {
    pub messages: Vec<Message>,
    pub input: String,
    pub input_mode: InputMode,
    pub cursor_position: usize,
    pub scroll_offset: usize,
    pub model: String,
    pub provider: String,
    pub api_keys: HashMap<String, String>,
    pub tokens_used: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost: f64,
    pub is_streaming: bool,
    pub should_quit: bool,
    pub pending_input: Option<String>,
    pub last_update: Instant,
    pub popup: PopupState,
    pub status_message: String,
}

impl App {
    pub fn new(model: String) -> Self {
        Self {
            messages: Vec::new(),
            input: String::new(),
            input_mode: InputMode::Normal,
            cursor_position: 0,
            scroll_offset: 0,
            model,
            provider: "anthropic".to_string(),
            api_keys: HashMap::new(),
            tokens_used: 0,
            input_tokens: 0,
            output_tokens: 0,
            cost: 0.0,
            is_streaming: false,
            should_quit: false,
            pending_input: None,
            last_update: Instant::now(),
            popup: PopupState::None,
            status_message: "Ready".to_string(),
        }
    }

    pub fn with_provider(mut self, provider: String) -> Self {
        self.provider = provider;
        self
    }

    pub fn with_api_key(mut self, provider: &str, api_key: String) -> Self {
        self.api_keys.insert(provider.to_string(), api_key);
        self
    }

    pub fn get_api_key(&self, provider: &str) -> Option<&String> {
        self.api_keys.get(provider)
    }

    pub fn show_provider_select(&mut self) {
        self.popup = PopupState::ProviderSelect { selected: 0 };
        self.input_mode = InputMode::ProviderSelect;
    }

    pub fn show_api_key_input(&mut self, provider_id: String) {
        self.popup = PopupState::ApiKeyInput {
            provider_id,
            api_key_input: String::new(),
        };
        self.input_mode = InputMode::ApiKeyInput;
    }

    pub fn close_popup(&mut self) {
        self.popup = PopupState::None;
        self.input_mode = InputMode::Normal;
    }

    pub fn add_system_message(&mut self, content: String, level: SystemLevel) {
        self.messages.push(Message::System { content, level });
        self.last_update = Instant::now();
    }

    pub fn clear_conversation(&mut self) {
        self.messages.clear();
        self.scroll_offset = 0;
        self.tokens_used = 0;
        self.input_tokens = 0;
        self.output_tokens = 0;
        self.cost = 0.0;
        self.add_system_message("Conversation cleared.".to_string(), SystemLevel::Info);
    }

    pub fn set_model(&mut self, model: String) {
        self.model = model;
        self.add_system_message(
            format!("Model changed to: {}", self.model),
            SystemLevel::Info,
        );
    }

    pub fn show_help(&mut self) {
        for line in crate::command::Command::help_text() {
            self.messages.push(Message::System {
                content: line,
                level: SystemLevel::Info,
            });
        }
        self.last_update = Instant::now();
    }

    pub fn handle_event(&mut self, event: AgentEvent) {
        match event {
            AgentEvent::TextDelta { text } => {
                if let Some(Message::Assistant { content, .. }) = self.messages.last_mut() {
                    content.push_str(&text);
                } else {
                    self.messages.push(Message::Assistant {
                        content: text,
                        thinking: None,
                    });
                }
            }
            AgentEvent::ThinkingDelta { thinking } => {
                if let Some(Message::Assistant { thinking: t, .. }) = self.messages.last_mut() {
                    t.get_or_insert_with(String::new).push_str(&thinking);
                }
            }
            AgentEvent::ToolCall { id, name, .. } => {
                self.messages.push(Message::ToolCall {
                    id,
                    name,
                    status: ToolStatus::Running,
                });
            }
            AgentEvent::ToolResult {
                id,
                result,
                is_error,
            } => {
                if let Some(Message::ToolCall { status, .. }) =
                    self.messages.iter_mut().rev().find(
                        |m| matches!(m, Message::ToolCall { id: tool_id, .. } if tool_id == &id),
                    )
                {
                    *status = if is_error {
                        ToolStatus::Failed
                    } else {
                        ToolStatus::Complete
                    };
                }
                self.messages.push(Message::ToolResult {
                    id,
                    output: result,
                    is_error,
                });
            }
            AgentEvent::Complete => {
                self.is_streaming = false;
            }
            AgentEvent::TokenUsage { input, output } => {
                self.input_tokens += input;
                self.output_tokens += output;
                self.tokens_used = self.input_tokens + self.output_tokens;
                if let Some(model) = crate::command::ModelInfo::find(&self.model) {
                    self.cost = model.calculate_cost(self.input_tokens, self.output_tokens);
                }
            }
            AgentEvent::Error(msg) => {
                self.messages.push(Message::Assistant {
                    content: format!("Error: {}", msg),
                    thinking: None,
                });
                self.is_streaming = false;
            }
            AgentEvent::StateChanged { to, .. } => {
                self.status_message = to;
            }
            AgentEvent::Cancelled => {
                self.is_streaming = false;
                self.status_message = "Cancelled".to_string();
            }
        }
        self.last_update = Instant::now();
    }

    pub fn scroll_up(&mut self) {
        if self.scroll_offset > 0 {
            self.scroll_offset -= 1;
        }
    }

    pub fn scroll_down(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_app() -> App {
        App::new("claude-sonnet-4-20250514".to_string())
    }

    #[test]
    fn test_app_new() {
        let app = create_test_app();
        assert_eq!(app.model, "claude-sonnet-4-20250514");
        assert_eq!(app.provider, "anthropic");
        assert!(app.messages.is_empty());
        assert!(app.input.is_empty());
        assert_eq!(app.input_mode, InputMode::Normal);
        assert_eq!(app.cursor_position, 0);
        assert_eq!(app.scroll_offset, 0);
        assert!(!app.should_quit);
        assert!(!app.is_streaming);
    }

    #[test]
    fn test_app_with_provider() {
        let app = create_test_app().with_provider("openrouter".to_string());
        assert_eq!(app.provider, "openrouter");
    }

    #[test]
    fn test_app_with_api_key() {
        let app = create_test_app().with_api_key("anthropic", "test-key".to_string());
        assert_eq!(app.get_api_key("anthropic"), Some(&"test-key".to_string()));
    }

    #[test]
    fn test_app_add_system_message() {
        let mut app = create_test_app();
        app.add_system_message("Test message".to_string(), SystemLevel::Info);
        assert_eq!(app.messages.len(), 1);
        match &app.messages[0] {
            Message::System { content, level } => {
                assert_eq!(content, "Test message");
                assert_eq!(*level, SystemLevel::Info);
            }
            _ => panic!("Expected System message"),
        }
    }

    #[test]
    fn test_app_system_levels() {
        let mut app = create_test_app();

        app.add_system_message("Info".to_string(), SystemLevel::Info);
        app.add_system_message("Warning".to_string(), SystemLevel::Warning);
        app.add_system_message("Error".to_string(), SystemLevel::Error);

        assert_eq!(app.messages.len(), 3);
    }

    #[test]
    fn test_app_clear_conversation() {
        let mut app = create_test_app();
        app.messages.push(Message::User {
            content: "Hello".to_string(),
        });
        app.tokens_used = 1000;
        app.input_tokens = 500;
        app.output_tokens = 500;
        app.cost = 0.05;

        app.clear_conversation();

        assert!(app.messages.len() == 1);
        assert_eq!(app.tokens_used, 0);
        assert_eq!(app.input_tokens, 0);
        assert_eq!(app.output_tokens, 0);
        assert_eq!(app.cost, 0.0);
        assert_eq!(app.scroll_offset, 0);
    }

    #[test]
    fn test_app_set_model() {
        let mut app = create_test_app();
        app.set_model("claude-3-opus".to_string());
        assert_eq!(app.model, "claude-3-opus");
    }

    #[test]
    fn test_app_show_provider_select() {
        let mut app = create_test_app();
        app.show_provider_select();
        assert_eq!(app.input_mode, InputMode::ProviderSelect);
        match &app.popup {
            PopupState::ProviderSelect { selected } => assert_eq!(*selected, 0),
            _ => panic!("Expected ProviderSelect popup"),
        }
    }

    #[test]
    fn test_app_show_api_key_input() {
        let mut app = create_test_app();
        app.show_api_key_input("openrouter".to_string());
        assert_eq!(app.input_mode, InputMode::ApiKeyInput);
        match &app.popup {
            PopupState::ApiKeyInput {
                provider_id,
                api_key_input,
            } => {
                assert_eq!(provider_id, "openrouter");
                assert!(api_key_input.is_empty());
            }
            _ => panic!("Expected ApiKeyInput popup"),
        }
    }

    #[test]
    fn test_app_close_popup() {
        let mut app = create_test_app();
        app.show_provider_select();
        app.close_popup();
        assert_eq!(app.input_mode, InputMode::Normal);
        assert!(matches!(app.popup, PopupState::None));
    }

    #[test]
    fn test_app_handle_text_delta() {
        let mut app = create_test_app();
        app.handle_event(AgentEvent::TextDelta {
            text: "Hello".to_string(),
        });

        assert_eq!(app.messages.len(), 1);
        match &app.messages[0] {
            Message::Assistant { content, thinking } => {
                assert_eq!(content, "Hello");
                assert!(thinking.is_none());
            }
            _ => panic!("Expected Assistant message"),
        }
    }

    #[test]
    fn test_app_handle_text_delta_append() {
        let mut app = create_test_app();
        app.handle_event(AgentEvent::TextDelta {
            text: "Hello".to_string(),
        });
        app.handle_event(AgentEvent::TextDelta {
            text: " World".to_string(),
        });

        match &app.messages[0] {
            Message::Assistant { content, .. } => assert_eq!(content, "Hello World"),
            _ => panic!("Expected Assistant message"),
        }
    }

    #[test]
    fn test_app_handle_thinking_delta() {
        let mut app = create_test_app();
        app.handle_event(AgentEvent::TextDelta {
            text: "Response".to_string(),
        });
        app.handle_event(AgentEvent::ThinkingDelta {
            thinking: "Let me think...".to_string(),
        });

        match &app.messages[0] {
            Message::Assistant { thinking, .. } => {
                assert_eq!(thinking, &Some("Let me think...".to_string()));
            }
            _ => panic!("Expected Assistant message"),
        }
    }

    #[test]
    fn test_app_handle_tool_call() {
        let mut app = create_test_app();
        app.handle_event(AgentEvent::ToolCall {
            id: "tool_123".to_string(),
            name: "read".to_string(),
            args: serde_json::json!({"path": "/test.txt"}),
        });

        assert_eq!(app.messages.len(), 1);
        match &app.messages[0] {
            Message::ToolCall { id, name, status } => {
                assert_eq!(id, "tool_123");
                assert_eq!(name, "read");
                assert_eq!(*status, ToolStatus::Running);
            }
            _ => panic!("Expected ToolCall message"),
        }
    }

    #[test]
    fn test_app_handle_tool_result() {
        let mut app = create_test_app();
        app.handle_event(AgentEvent::ToolCall {
            id: "tool_123".to_string(),
            name: "read".to_string(),
            args: serde_json::json!({}),
        });
        app.handle_event(AgentEvent::ToolResult {
            id: "tool_123".to_string(),
            result: "File contents".to_string(),
            is_error: false,
        });

        assert_eq!(app.messages.len(), 2);
        match &app.messages[1] {
            Message::ToolResult {
                id,
                output,
                is_error,
            } => {
                assert_eq!(id, "tool_123");
                assert_eq!(output, "File contents");
                assert!(!is_error);
            }
            _ => panic!("Expected ToolResult message"),
        }
    }

    #[test]
    fn test_app_handle_tool_result_error() {
        let mut app = create_test_app();
        app.handle_event(AgentEvent::ToolCall {
            id: "tool_123".to_string(),
            name: "bash".to_string(),
            args: serde_json::json!({}),
        });
        app.handle_event(AgentEvent::ToolResult {
            id: "tool_123".to_string(),
            result: "Command failed".to_string(),
            is_error: true,
        });

        match &app.messages[0] {
            Message::ToolCall { status, .. } => assert_eq!(*status, ToolStatus::Failed),
            _ => panic!("Expected ToolCall"),
        }
        match &app.messages[1] {
            Message::ToolResult { is_error, .. } => assert!(is_error),
            _ => panic!("Expected ToolResult"),
        }
    }

    #[test]
    fn test_app_handle_complete() {
        let mut app = create_test_app();
        app.is_streaming = true;
        app.handle_event(AgentEvent::Complete);
        assert!(!app.is_streaming);
    }

    #[test]
    fn test_app_handle_token_usage() {
        let mut app = create_test_app();
        app.handle_event(AgentEvent::TokenUsage {
            input: 100,
            output: 50,
        });

        assert_eq!(app.input_tokens, 100);
        assert_eq!(app.output_tokens, 50);
        assert_eq!(app.tokens_used, 150);
    }

    #[test]
    fn test_app_handle_error() {
        let mut app = create_test_app();
        app.is_streaming = true;
        app.handle_event(AgentEvent::Error("Something went wrong".to_string()));

        assert!(!app.is_streaming);
        match &app.messages[0] {
            Message::Assistant { content, .. } => {
                assert!(content.contains("Error"));
                assert!(content.contains("Something went wrong"));
            }
            _ => panic!("Expected Assistant message with error"),
        }
    }

    #[test]
    fn test_app_scroll() {
        let mut app = create_test_app();

        app.scroll_down();
        assert_eq!(app.scroll_offset, 1);

        app.scroll_down();
        assert_eq!(app.scroll_offset, 2);

        app.scroll_up();
        assert_eq!(app.scroll_offset, 1);

        app.scroll_up();
        assert_eq!(app.scroll_offset, 0);

        app.scroll_up();
        assert_eq!(app.scroll_offset, 0);
    }

    #[test]
    fn test_input_mode_equality() {
        assert_eq!(InputMode::Normal, InputMode::Normal);
        assert_eq!(InputMode::Insert, InputMode::Insert);
        assert_ne!(InputMode::Normal, InputMode::Insert);
    }

    #[test]
    fn test_tool_status_equality() {
        assert_eq!(ToolStatus::Pending, ToolStatus::Pending);
        assert_eq!(ToolStatus::Running, ToolStatus::Running);
        assert_ne!(ToolStatus::Pending, ToolStatus::Complete);
    }

    #[test]
    fn test_system_level_equality() {
        assert_eq!(SystemLevel::Info, SystemLevel::Info);
        assert_eq!(SystemLevel::Warning, SystemLevel::Warning);
        assert_ne!(SystemLevel::Info, SystemLevel::Error);
    }
}
