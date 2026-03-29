use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub enum AgentState {
    Idle,
    Planning,
    Executing { iteration: usize },
    Verifying,
    Complete,
    Error(String),
    Cancelled,
}

impl std::fmt::Display for AgentState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentState::Idle => write!(f, "Idle"),
            AgentState::Planning => write!(f, "Planning"),
            AgentState::Executing { iteration } => write!(f, "Executing (iteration {})", iteration),
            AgentState::Verifying => write!(f, "Verifying"),
            AgentState::Complete => write!(f, "Complete"),
            AgentState::Error(msg) => write!(f, "Error: {}", msg),
            AgentState::Cancelled => write!(f, "Cancelled"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum AgentEvent {
    TextDelta {
        text: String,
    },
    ThinkingDelta {
        thinking: String,
    },
    ToolCall {
        id: String,
        name: String,
        args: Value,
    },
    ToolResult {
        id: String,
        result: String,
        is_error: bool,
    },
    TokenUsage {
        input: u64,
        output: u64,
    },
    StateChanged {
        from: String,
        to: String,
    },
    ContextBudgetWarning {
        usage_percent: f64,
    },
    OutputTruncated {
        tool_name: String,
        original_len: usize,
        truncated_len: usize,
    },
    Cancelled,
    Complete,
    Error(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_delta_event() {
        let event = AgentEvent::TextDelta {
            text: "Hello".to_string(),
        };

        match event {
            AgentEvent::TextDelta { text } => assert_eq!(text, "Hello"),
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_thinking_delta_event() {
        let event = AgentEvent::ThinkingDelta {
            thinking: "Processing...".to_string(),
        };

        match event {
            AgentEvent::ThinkingDelta { thinking } => assert_eq!(thinking, "Processing..."),
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_tool_call_event() {
        let event = AgentEvent::ToolCall {
            id: "tool_123".to_string(),
            name: "read".to_string(),
            args: serde_json::json!({"path": "/test.txt"}),
        };

        match event {
            AgentEvent::ToolCall { id, name, args } => {
                assert_eq!(id, "tool_123");
                assert_eq!(name, "read");
                assert_eq!(args["path"], "/test.txt");
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_tool_result_event() {
        let event = AgentEvent::ToolResult {
            id: "tool_123".to_string(),
            result: "File contents".to_string(),
            is_error: false,
        };

        match event {
            AgentEvent::ToolResult {
                id,
                result,
                is_error,
            } => {
                assert_eq!(id, "tool_123");
                assert_eq!(result, "File contents");
                assert!(!is_error);
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_token_usage_event() {
        let event = AgentEvent::TokenUsage {
            input: 100,
            output: 50,
        };

        match event {
            AgentEvent::TokenUsage { input, output } => {
                assert_eq!(input, 100);
                assert_eq!(output, 50);
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_complete_event() {
        let event = AgentEvent::Complete;

        match event {
            AgentEvent::Complete => (),
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_context_budget_warning_event() {
        let event = AgentEvent::ContextBudgetWarning {
            usage_percent: 85.5,
        };

        match event {
            AgentEvent::ContextBudgetWarning { usage_percent } => {
                assert!((usage_percent - 85.5).abs() < 0.01);
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_output_truncated_event() {
        let event = AgentEvent::OutputTruncated {
            tool_name: "bash".to_string(),
            original_len: 10000,
            truncated_len: 5000,
        };

        match event {
            AgentEvent::OutputTruncated {
                tool_name,
                original_len,
                truncated_len,
            } => {
                assert_eq!(tool_name, "bash");
                assert_eq!(original_len, 10000);
                assert_eq!(truncated_len, 5000);
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_error_event() {
        let event = AgentEvent::Error("Something went wrong".to_string());

        match event {
            AgentEvent::Error(msg) => assert_eq!(msg, "Something went wrong"),
            _ => panic!("Wrong event type"),
        }
    }
}
