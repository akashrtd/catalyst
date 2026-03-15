mod anthropic;
mod openrouter;
mod provider;
mod types;

pub use anthropic::AnthropicClient;
pub use openrouter::OpenRouterClient;
pub use provider::{LlmProvider, LlmStream};
pub use types::*;

pub type Result<T> = anyhow::Result<T>;

#[derive(Debug, Clone, PartialEq)]
pub enum Provider {
    Anthropic,
    OpenRouter,
}

impl Provider {
    pub fn from_string(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "anthropic" | "claude" => Some(Self::Anthropic),
            "openrouter" | "or" => Some(Self::OpenRouter),
            _ => None,
        }
    }
}

pub fn create_provider(provider: Provider, api_key: String, model: String) -> Box<dyn LlmProvider> {
    match provider {
        Provider::Anthropic => Box::new(AnthropicClient::new(api_key, model)),
        Provider::OpenRouter => Box::new(OpenRouterClient::new(api_key, model)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_from_string() {
        assert_eq!(
            Provider::from_string("anthropic"),
            Some(Provider::Anthropic)
        );
        assert_eq!(
            Provider::from_string("ANTHROPIC"),
            Some(Provider::Anthropic)
        );
        assert_eq!(Provider::from_string("claude"), Some(Provider::Anthropic));
        assert_eq!(
            Provider::from_string("openrouter"),
            Some(Provider::OpenRouter)
        );
        assert_eq!(Provider::from_string("or"), Some(Provider::OpenRouter));
        assert_eq!(Provider::from_string("unknown"), None);
    }

    #[test]
    fn test_role_serialization() {
        let role = Role::User;
        let json = serde_json::to_string(&role).unwrap();
        assert_eq!(json, "\"user\"");

        let role = Role::Assistant;
        let json = serde_json::to_string(&role).unwrap();
        assert_eq!(json, "\"assistant\"");
    }

    #[test]
    fn test_message_serialization() {
        let msg = Message {
            role: Role::User,
            content: Content::Text("Hello".to_string()),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"role\":\"user\""));
        assert!(json.contains("\"content\":\"Hello\""));
    }

    #[test]
    fn test_content_block_text() {
        let block = ContentBlock::Text {
            text: "Hello world".to_string(),
        };

        let json = serde_json::to_string(&block).unwrap();
        assert!(json.contains("\"type\":\"text\""));
        assert!(json.contains("\"text\":\"Hello world\""));
    }

    #[test]
    fn test_content_block_thinking() {
        let block = ContentBlock::Thinking {
            thinking: "Thinking...".to_string(),
        };

        let json = serde_json::to_string(&block).unwrap();
        assert!(json.contains("\"type\":\"thinking\""));
        assert!(json.contains("\"thinking\":\"Thinking...\""));
    }

    #[test]
    fn test_content_block_tool_use() {
        let block = ContentBlock::ToolUse {
            id: "tool_123".to_string(),
            name: "read".to_string(),
            input: serde_json::json!({"path": "/test.txt"}),
        };

        let json = serde_json::to_string(&block).unwrap();
        assert!(json.contains("\"type\":\"tool_use\""));
        assert!(json.contains("\"id\":\"tool_123\""));
        assert!(json.contains("\"name\":\"read\""));
    }

    #[test]
    fn test_content_block_tool_result() {
        let block = ContentBlock::ToolResult {
            tool_use_id: "tool_123".to_string(),
            content: "File contents".to_string(),
            is_error: false,
        };

        let json = serde_json::to_string(&block).unwrap();
        assert!(json.contains("\"type\":\"tool_result\""));
        assert!(json.contains("\"tool_use_id\":\"tool_123\""));
        assert!(json.contains("\"is_error\":false"));
    }

    #[test]
    fn test_tool_serialization() {
        let tool = ToolDef {
            name: "read".to_string(),
            description: "Read a file".to_string(),
            input_schema: serde_json::json!({"type": "object"}),
        };

        let json = serde_json::to_string(&tool).unwrap();
        assert!(json.contains("\"name\":\"read\""));
        assert!(json.contains("\"description\":\"Read a file\""));
    }
}
