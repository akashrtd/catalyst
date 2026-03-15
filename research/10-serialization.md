# Serialization Strategy

## Requirements

- LLM API request/response parsing (JSON)
- Configuration files (TOML)
- Session persistence (JSON/binary)
- Performance for large files

## Crates

### serde ⭐⭐⭐⭐⭐ (Foundation)

The standard serialization framework.

```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
```

```rust
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}
```

### serde_json ⭐⭐⭐⭐⭐ (JSON)

For LLM API communication.

```rust
use serde_json::{json, Value};

let request = json!({
    "model": "claude-sonnet-4-20250514",
    "messages": messages,
});

let response: Response = serde_json::from_str(&text)?;
```

### toml ⭐⭐⭐⭐⭐ (Config Files)

For configuration.

```rust
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    pub llm: LlmConfig,
    pub ui: UiConfig,
}

#[derive(Deserialize)]
pub struct LlmConfig {
    pub provider: String,
    pub model: String,
    pub api_key: Option<String>,
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = dirs::config_dir()
            .unwrap()
            .join("catalyst/config.toml");
        let content = fs::read_to_string(path)?;
        toml::from_str(&content).context("Invalid config")
    }
}
```

### simd-json ⭐⭐⭐⭐ (Fast JSON)

SIMD-accelerated JSON parsing.

**Pros:**
- 2-3x faster than serde_json
- Same API as serde_json

**Cons:**
- Requires x86_64 or ARM64
- Slightly larger binary

```toml
[target.'cfg(any(target_arch = "x86_64", target_arch = "aarch64"))'.dependencies]
simd-json = "0.13"
```

## Data Structures

### API Types (LLM Communication)

```rust
// catalyst-llm/src/types.rs
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct MessageRequest {
    pub model: String,
    pub max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    pub messages: Vec<Message>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<Tool>,
    #[serde(default)]
    pub stream: bool,
}

#[derive(Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: Content,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum Content {
    Text(String),
    Blocks(Vec<ContentBlock>),
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: String,
        #[serde(default)]
        is_error: bool,
    },
}

#[derive(Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}
```

### Streaming Events

```rust
#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum StreamEvent {
    #[serde(rename = "message_start")]
    MessageStart { message: MessageInfo },
    
    #[serde(rename = "content_block_start")]
    ContentBlockStart {
        index: usize,
        content_block: ContentBlock,
    },
    
    #[derive(Deserialize)]
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta {
        index: usize,
        delta: Delta,
    },
    
    #[serde(rename = "content_block_stop")]
    ContentBlockStop { index: usize },
    
    #[serde(rename = "message_delta")]
    MessageDelta {
        delta: MessageDeltaInfo,
        usage: Usage,
    },
    
    #[serde(rename = "message_stop")]
    MessageStop,
    
    #[serde(rename = "error")]
    Error { error: ApiError },
}

#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum Delta {
    #[serde(rename = "text_delta")]
    TextDelta { text: String },
    
    #[serde(rename = "input_json_delta")]
    InputJsonDelta { partial_json: String },
    
    #[serde(rename = "thinking_delta")]
    ThinkingDelta { thinking: String },
}
```

### Configuration

```rust
// catalyst-config/src/lib.rs
use serde::Deserialize;

#[derive(Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub llm: LlmConfig,
    #[serde(default)]
    pub ui: UiConfig,
    #[serde(default)]
    pub tools: ToolConfig,
}

#[derive(Deserialize)]
pub struct LlmConfig {
    #[serde(default = "default_provider")]
    pub provider: String,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default)]
    pub api_key_env: String,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default)]
    pub temperature: Option<f32>,
}

fn default_provider() -> String { "anthropic".into() }
fn default_model() -> String { "claude-sonnet-4-20250514".into() }
fn default_max_tokens() -> u32 { 4096 }

#[derive(Deserialize)]
pub struct UiConfig {
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default)]
    pub show_token_usage: bool,
    #[serde(default)]
    pub show_thinking: bool,
}

fn default_theme() -> String { "dark".into() }
```

### Session Persistence

```rust
// catalyst-core/src/session.rs
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub created_at: i64,
    pub messages: Vec<StoredMessage>,
    pub metadata: SessionMetadata,
}

#[derive(Serialize, Deserialize)]
pub struct StoredMessage {
    pub role: Role,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<StoredToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_result: Option<StoredToolResult>,
}

#[derive(Serialize, Deserialize)]
pub struct SessionMetadata {
    pub model: String,
    pub total_tokens: u64,
    pub total_cost: f64,
    pub working_dir: String,
}

impl Session {
    pub fn save(&self, path: &Path) -> Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }
    
    pub fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let session = serde_json::from_str(&content)?;
        Ok(session)
    }
}
```

## Best Practices

1. **Use `#[serde(default)]`** - For backward compatibility
2. **Use `#[serde(skip_serializing_if = "...")]`** - Reduce payload size
3. **Use `#[serde(rename_all = "...")]`** - Match API conventions
4. **Use `#[serde(untagged)]`** - For union types
5. **Use `#[serde(tag = "type")]`** - For tagged enums

## Cargo.toml

```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
toml = "0.8"
```

## Optional: Performance

```toml
# Faster JSON for large payloads
[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
simd-json = "0.13"
```
