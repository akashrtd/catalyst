# LLM Integration - Anthropic

## MVP Scope

Start with Anthropic Claude API only. Future phases will add more providers.

## Why Anthropic First (March 2026)

| Reason | Details |
|--------|---------|
| Best coding model | Claude 4 Sonnet/Opus lead coding benchmarks |
| Tool calling | Excellent function calling support |
| Long context | 200K tokens for large codebases |
| Streaming | Native streaming support |
| Extended thinking | Claude 4 has best reasoning transparency |
| Prompt caching | Reduces costs significantly |
| Documentation | Well-documented API |
| Reliability | Consistent behavior, good uptime |

## Claude 4 Models (2026)

| Model | Context | Best For | Cost (per 1M tokens) |
|-------|---------|----------|---------------------|
| claude-sonnet-4-20250514 | 200K | General coding, fast | $3 input / $15 output |
| claude-opus-4-20250514 | 200K | Complex reasoning | $15 input / $75 output |

## API Details

### Endpoint
```
POST https://api.anthropic.com/v1/messages
```

### Authentication
```
Header: x-api-key: sk-ant-...
Header: anthropic-version: 2023-06-01
```

### Request Structure

```json
{
  "model": "claude-sonnet-4-20250514",
  "max_tokens": 4096,
  "system": "You are Catalyst, a research-driven AI coding agent...",
  "messages": [
    {"role": "user", "content": "Read the main.rs file"}
  ],
  "tools": [
    {
      "name": "read",
      "description": "Read a file from the filesystem",
      "input_schema": {
        "type": "object",
        "properties": {
          "path": {"type": "string", "description": "File path to read"}
        },
        "required": ["path"]
      }
    }
  ],
  "stream": true
}
```

### Response Structure (Streaming)

```
event: message_start
data: {"type": "message_start", "message": {...}}

event: content_block_start
data: {"type": "content_block_start", "index": 0, "content_block": {"type": "text", "text": ""}}

event: content_block_delta
data: {"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": "I'll"}}

event: content_block_delta
data: {"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": " read"}}

event: content_block_stop
data: {"type": "content_block_stop", "index": 0}

event: message_stop
data: {"type": "message_stop"}
```

### Tool Call Response

```json
{
  "type": "content_block_start",
  "content_block": {
    "type": "tool_use",
    "id": "toolu_01...",
    "name": "read",
    "input": {}
  }
}
```

## Rust Implementation

### Crate Selection

```
reqwest       - HTTP client with streaming
reqwest-eventsource - SSE handling
serde         - JSON serialization
tokio         - Async runtime
```

### Client Structure

```rust
pub struct AnthropicClient {
    http: reqwest::Client,
    api_key: String,
    model: String,
}

pub struct Message {
    pub role: Role,
    pub content: Vec<ContentBlock>,
}

pub enum ContentBlock {
    Text { text: String },
    ToolUse { id: String, name: String, input: Value },
    ToolResult { tool_use_id: String, content: String, is_error: bool },
}

pub enum Role {
    User,
    Assistant,
}
```

### Streaming Implementation

```rust
impl AnthropicClient {
    pub async fn stream(
        &self,
        messages: Vec<Message>,
        tools: Vec<Tool>,
    ) -> impl Stream<Item = Result<StreamEvent, Error>> {
        let response = self.http
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&Request {
                model: &self.model,
                messages,
                tools,
                stream: true,
                ..Default::default()
            })
            .send()
            .await?;

        // Parse SSE stream
        parse_sse_stream(response)
    }
}

pub enum StreamEvent {
    MessageStart { message: MessageInfo },
    ContentBlockStart { index: usize, block: ContentBlock },
    ContentDelta { index: usize, delta: Delta },
    ContentBlockStop { index: usize },
    MessageStop,
    Error { error: ApiError },
}
```

### Integration with TUI

```rust
// In App::handle_llm_stream()
pub async fn process_stream(&mut self, mut stream: impl Stream<Item = StreamEvent>) {
    while let Some(event) = stream.next().await {
        match event {
            StreamEvent::ContentDelta { delta, .. } => {
                if let Delta::Text(text) = delta {
                    self.append_assistant_text(&text);
                    self.render(); // Trigger immediate render
                }
            }
            StreamEvent::ContentBlockStart { block, .. } => {
                if let ContentBlock::ToolUse { name, .. } = block {
                    self.add_tool_call(&name);
                }
            }
            _ => {}
        }
    }
}
```

## Models

| Model | Context | Best For |
|-------|---------|----------|
| claude-sonnet-4-20250514 | 200K | General coding, fast |
| claude-opus-4-20250514 | 200K | Complex reasoning |

## Error Handling

```rust
pub enum ApiError {
    Authentication,
    RateLimit { retry_after: Duration },
    ContextLengthExceeded,
    InvalidRequest(String),
    Overloaded,
    Network(reqwest::Error),
}

impl ApiError {
    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::RateLimit { .. } | Self::Overloaded | Self::Network(_))
    }
}
```

## Configuration

```toml
# ~/.catalyst/config.toml
[llm]
provider = "anthropic"
model = "claude-sonnet-4-20250514"
api_key_env = "ANTHROPIC_API_KEY"
max_tokens = 4096
temperature = 1.0

[llm.cache]
# Anthropic prompt caching
system = true
tools = true
messages = "auto"  # Let Catalyst decide
```

## Future: Multi-Provider

Phase 4 will add:
- OpenAI
- Google Gemini
- Local models (via OpenAI-compatible API)
- Custom providers

Abstract behind `LLMProvider` trait:

```rust
#[async_trait]
pub trait LLMProvider {
    async fn stream(&self, request: Request) -> Result<Stream, Error>;
    fn models(&self) -> Vec<ModelInfo>;
    fn name(&self) -> &str;
}
```

## Resources

- [Anthropic API docs](https://docs.anthropic.com)
- [Messages API](https://docs.anthropic.com/en/api/messages)
- [Tool use](https://docs.anthropic.com/en/docs/tool-use)
- [Streaming](https://docs.anthropic.com/en/api/messages-streaming)
