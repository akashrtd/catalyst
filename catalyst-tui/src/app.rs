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
