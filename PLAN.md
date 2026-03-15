# Catalyst MVP Implementation Plan

## Overview

This document outlines the detailed implementation plan for Catalyst MVP - a minimal, research-driven AI coding CLI with TUI support.

## MVP Scope

### In Scope
- Multi-crate Rust workspace
- TUI with ratatui + crossterm
- Anthropic Claude 4 API integration
- Streaming responses with extended thinking
- 4 core tools: read, write, edit, bash
- Slash commands: /help, /model, /clear, /exit
- Event-driven async architecture
- Configuration via TOML

### Out of Scope (Future)
- TLA+ verification
- Simulation engine
- Multi-provider LLM support
- Session persistence
- File references (@file)
- Extensions/skills system

---

## Phase 1: Project Setup (Day 1)

### 1.1 Initialize Workspace

```bash
# Create workspace root
mkdir -p catalyst/{catalyst-cli,catalyst-core,catalyst-llm,catalyst-tools,catalyst-tui}/src

# Initialize git
git init
echo "target/" > .gitignore
echo "Cargo.lock" >> .gitignore
```

### 1.2 Create Workspace Cargo.toml

**File:** `Cargo.toml`

```toml
[workspace]
members = [
    "catalyst-cli",
    "catalyst-core",
    "catalyst-llm",
    "catalyst-tools",
    "catalyst-tui",
]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "MIT"
repository = "https://github.com/catalyst/catalyst"
authors = ["Catalyst Team"]

[workspace.dependencies]
# Async
tokio = { version = "1", features = ["full"] }
futures = "0.3"

# TUI
ratatui = "0.29"
crossterm = { version = "0.28", features = ["event-stream"] }

# HTTP & Serialization
reqwest = { version = "0.12", features = ["json", "stream"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
toml = "0.8"

# Error handling
anyhow = "1.0"
thiserror = "2.0"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Utilities
dirs = "6.0"
uuid = { version = "1.11", features = ["v4"] }
regex = "1.11"

# Internal crates
catalyst-core = { path = "catalyst-core" }
catalyst-llm = { path = "catalyst-llm" }
catalyst-tools = { path = "catalyst-tools" }
catalyst-tui = { path = "catalyst-tui" }
```

### 1.3 Create Crate Cargo.toml Files

**File:** `catalyst-cli/Cargo.toml`
```toml
[package]
name = "catalyst-cli"
version.workspace = true
edition.workspace = true

[[bin]]
name = "catalyst"
path = "src/main.rs"

[dependencies]
catalyst-core.workspace = true
catalyst-llm.workspace = true
catalyst-tools.workspace = true
catalyst-tui.workspace = true

tokio.workspace = true
anyhow.workspace = true
tracing.workspace = true
tracing-subscriber.workspace = true
clap = { version = "4.5", features = ["derive"] }
```

**File:** `catalyst-core/Cargo.toml`
```toml
[package]
name = "catalyst-core"
version.workspace = true
edition.workspace = true

[dependencies]
catalyst-llm.workspace = true
catalyst-tools.workspace = true

tokio.workspace = true
futures.workspace = true
serde.workspace = true
serde_json.workspace = true
anyhow.workspace = true
thiserror.workspace = true
tracing.workspace = true
uuid.workspace = true
```

**File:** `catalyst-llm/Cargo.toml`
```toml
[package]
name = "catalyst-llm"
version.workspace = true
edition.workspace = true

[dependencies]
tokio.workspace = true
futures.workspace = true
reqwest.workspace = true
serde.workspace = true
serde_json.workspace = true
anyhow.workspace = true
thiserror.workspace = true
tracing.workspace = true
```

**File:** `catalyst-tools/Cargo.toml`
```toml
[package]
name = "catalyst-tools"
version.workspace = true
edition.workspace = true

[dependencies]
tokio.workspace = true
serde.workspace = true
serde_json.workspace = true
anyhow.workspace = true
thiserror.workspace = true
tracing.workspace = true
regex.workspace = true
```

**File:** `catalyst-tui/Cargo.toml`
```toml
[package]
name = "catalyst-tui"
version.workspace = true
edition.workspace = true

[dependencies]
catalyst-core.workspace = true

tokio.workspace = true
ratatui.workspace = true
crossterm.workspace = true
anyhow.workspace = true
thiserror.workspace = true
tracing.workspace = true
```

### 1.4 Create Source Files

```bash
# Create lib.rs files
touch catalyst-core/src/lib.rs
touch catalyst-llm/src/lib.rs
touch catalyst-tools/src/lib.rs
touch catalyst-tui/src/lib.rs

# Create main.rs
touch catalyst-cli/src/main.rs
```

### 1.5 Verify Build

```bash
cargo build --workspace
```

**Deliverable:** Compiling workspace with empty crates

---

## Phase 2: LLM Client (Days 2-3)

### 2.1 API Types

**File:** `catalyst-llm/src/types.rs`

```rust
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: Content,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Content {
    Text(String),
    Blocks(Vec<ContentBlock>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    
    #[serde(rename = "thinking")]
    Thinking { thinking: String },
    
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

#[derive(Debug, Clone, Serialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Clone, Deserialize)]
pub struct MessageResponse {
    pub id: String,
    pub role: Role,
    pub content: Vec<ContentBlock>,
    pub model: String,
    pub usage: Usage,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    #[serde(default)]
    pub cache_creation_input_tokens: u64,
    #[serde(default)]
    pub cache_read_input_tokens: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum StreamEvent {
    #[serde(rename = "message_start")]
    MessageStart { message: MessageInfo },
    
    #[serde(rename = "content_block_start")]
    ContentBlockStart {
        index: usize,
        content_block: ContentBlock,
    },
    
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

#[derive(Debug, Clone, Deserialize)]
pub struct MessageInfo {
    pub id: String,
    pub model: String,
    pub role: Role,
    pub content: Vec<ContentBlock>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MessageDeltaInfo {
    #[serde(default)]
    pub stop_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum Delta {
    #[serde(rename = "text_delta")]
    TextDelta { text: String },
    
    #[serde(rename = "thinking_delta")]
    ThinkingDelta { thinking: String },
    
    #[serde(rename = "input_json_delta")]
    InputJsonDelta { partial_json: String },
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiError {
    #[serde(rename = "type")]
    pub error_type: String,
    pub message: String,
}
```

### 2.2 Client Implementation

**File:** `catalyst-llm/src/client.rs`

```rust
use anyhow::{Context, Result};
use futures::{Stream, StreamExt};
use reqwest::Client;
use serde_json::json;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

use crate::types::*;

pub struct AnthropicClient {
    http: Client,
    api_key: String,
    model: String,
    base_url: String,
}

impl AnthropicClient {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            http: Client::new(),
            api_key,
            model,
            base_url: "https://api.anthropic.com".to_string(),
        }
    }
    
    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.base_url = base_url;
        self
    }
    
    pub async fn stream(
        &self,
        system: Option<&str>,
        messages: Vec<Message>,
        tools: Vec<Tool>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamEvent>> + Send>>> {
        let request = MessageRequest {
            model: self.model.clone(),
            max_tokens: 4096,
            system: system.map(|s| s.to_string()),
            messages,
            tools,
            stream: true,
        };
        
        let response = self.http
            .post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Anthropic API")?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("API error ({}): {}", status, body);
        }
        
        Ok(Box::pin(parse_sse_stream(response)))
    }
}

struct StreamState {
    response: reqwest::Response,
    buffer: String,
}

fn parse_sse_stream(
    response: reqwest::Response,
) -> impl Stream<Item = Result<StreamEvent>> {
    let state = Arc::new(Mutex::new(StreamState {
        response,
        buffer: String::new(),
    }));
    
    futures::stream::unfold(state, |state| async move {
        let mut guard = state.lock().await;
        
        loop {
            // Try to parse complete events from buffer first
            while let Some(pos) = guard.buffer.find("\n\n") {
                let event_str = guard.buffer.drain(..pos + 2).collect::<String>();
                
                for line in event_str.lines() {
                    if let Some(json_str) = line.strip_prefix("data: ") {
                        if json_str.is_empty() {
                            continue;
                        }
                        
                        match serde_json::from_str::<StreamEvent>(json_str) {
                            Ok(event) => {
                                return Some((Ok(event), state));
                            }
                            Err(e) => {
                                debug!("Failed to parse event: {} - {}", e, json_str);
                                continue;
                            }
                        }
                    }
                }
            }
            
            // Need more data
            match guard.response.chunk().await {
                Ok(Some(chunk)) => {
                    guard.buffer.push_str(&String::from_utf8_lossy(&chunk));
                }
                Ok(None) => {
                    // Stream ended, process remaining buffer
                    if guard.buffer.is_empty() {
                        return None;
                    }
                    // Try to parse remaining data
                    let remaining = guard.buffer.drain(..).collect::<String>();
                    debug!("Stream ended with unprocessed data: {}", remaining);
                    return None;
                }
                Err(e) => {
                    return Some((Err(anyhow::anyhow!("Stream error: {}", e)), state));
                }
            }
        }
    })
}
```

### 2.3 Module Exports

**File:** `catalyst-llm/src/lib.rs`

```rust
mod client;
mod types;

pub use client::*;
pub use types::*;

pub type Result<T> = anyhow::Result<T>;
```

**Deliverable:** Working LLM client with streaming

---

## Phase 3: Tool System (Days 4-5)

### 3.1 Tool Trait

**File:** `catalyst-tools/src/lib.rs`

```rust
mod registry;
mod read;
mod write;
mod edit;
mod bash;

pub use registry::*;
pub use read::*;
pub use write::*;
pub use edit::*;
pub use bash::*;

use anyhow::Result;
use serde_json::{json, Value};
use std::collections::HashMap;

pub struct ToolContext {
    pub working_dir: std::path::PathBuf,
    pub env: HashMap<String, String>,
    pub timeout_ms: u64,
}

pub struct ToolResult {
    pub output: String,
    pub metadata: HashMap<String, Value>,
}

impl ToolResult {
    pub fn success(output: impl Into<String>) -> Self {
        Self {
            output: output.into(),
            metadata: HashMap::new(),
        }
    }
    
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            output: format!("Error: {}", message.into()),
            metadata: HashMap::new(),
        }
    }
}

pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> Value;
    fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult>;
}
```

### 3.2 Read Tool

**File:** `catalyst-tools/src/read.rs`

```rust
use super::*;
use anyhow::{bail, Context};
use std::fs;

pub struct ReadTool;

impl Tool for ReadTool {
    fn name(&self) -> &str {
        "read"
    }
    
    fn description(&self) -> &str {
        "Read a file from the filesystem. Returns the file contents with line numbers."
    }
    
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The absolute or relative path to the file to read"
                },
                "offset": {
                    "type": "integer",
                    "description": "Line number to start reading from (1-indexed)",
                    "default": 1
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of lines to read",
                    "default": 2000
                }
            },
            "required": ["path"]
        })
    }
    
    fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult> {
        let path = args["path"].as_str()
            .context("path required")?;
        
        let path = ctx.working_dir.join(path);
        
        // Validate path is within working directory (security)
        let canonical_path = path.canonicalize()
            .with_context(|| format!("Path does not exist: {}", path.display()))?;
        let canonical_working_dir = ctx.working_dir.canonicalize()
            .context("Working directory does not exist")?;
        
        if !canonical_path.starts_with(&canonical_working_dir) {
            bail!("Path '{}' is outside working directory", canonical_path.display());
        }
        
        let offset = args.get("offset")
            .and_then(|v| v.as_u64())
            .unwrap_or(1) as usize;
        
        let limit = args.get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(2000) as usize;
        
        // Use blocking I/O (tools run in spawn_blocking context)
        let content = fs::read_to_string(&canonical_path)
            .with_context(|| format!("Failed to read file: {}", canonical_path.display()))?;
        
        let lines: Vec<_> = content
            .lines()
            .skip(offset.saturating_sub(1))
            .take(limit)
            .enumerate()
            .map(|(i, line)| format!("{:>6}\t{}", offset + i, line))
            .collect();
        
        Ok(ToolResult::success(lines.join("\n")))
    }
}
```

### 3.3 Write Tool

**File:** `catalyst-tools/src/write.rs`

```rust
use super::*;
use anyhow::{bail, Context};
use std::fs;

pub struct WriteTool;

impl Tool for WriteTool {
    fn name(&self) -> &str {
        "write"
    }
    
    fn description(&self) -> &str {
        "Write content to a new file. Fails if file already exists."
    }
    
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The path where the file should be created"
                },
                "content": {
                    "type": "string",
                    "description": "The content to write to the file"
                }
            },
            "required": ["path", "content"]
        })
    }
    
    fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult> {
        let path = args["path"].as_str()
            .context("path required")?;
        let content = args["content"].as_str()
            .context("content required")?;
        
        let path = ctx.working_dir.join(path);
        
        if path.exists() {
            bail!("File already exists: {}. Use edit tool instead.", path.display());
        }
        
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .context("Failed to create parent directories")?;
        }
        
        fs::write(&path, content)
            .with_context(|| format!("Failed to write file: {}", path.display()))?;
        
        Ok(ToolResult::success(format!("File created: {}", path.display())))
    }
}
```

### 3.4 Edit Tool

**File:** `catalyst-tools/src/edit.rs`

```rust
use super::*;
use anyhow::{bail, Context};
use std::fs;

pub struct EditTool;

impl Tool for EditTool {
    fn name(&self) -> &str {
        "edit"
    }
    
    fn description(&self) -> &str {
        "Edit an existing file by replacing specific text. The old_string must match exactly."
    }
    
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The path to the file to edit"
                },
                "old_string": {
                    "type": "string",
                    "description": "The exact text to find and replace"
                },
                "new_string": {
                    "type": "string",
                    "description": "The text to replace it with"
                },
                "replace_all": {
                    "type": "boolean",
                    "description": "Replace all occurrences",
                    "default": false
                }
            },
            "required": ["path", "old_string", "new_string"]
        })
    }
    
    fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult> {
        let path = args["path"].as_str()
            .context("path required")?;
        let old_string = args["old_string"].as_str()
            .context("old_string required")?;
        let new_string = args["new_string"].as_str()
            .context("new_string required")?;
        let replace_all = args.get("replace_all")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        
        let path = ctx.working_dir.join(path);
        
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read file: {}", path.display()))?;
        
        let occurrences = content.matches(old_string).count();
        
        if occurrences == 0 {
            bail!("old_string not found in file: {}", path.display());
        }
        
        if occurrences > 1 && !replace_all {
            bail!(
                "Found {} occurrences. Set replace_all=true or provide more context.",
                occurrences
            );
        }
        
        let new_content = if replace_all {
            content.replace(old_string, new_string)
        } else {
            content.replacen(old_string, new_string, 1)
        };
        
        fs::write(&path, &new_content)
            .with_context(|| format!("Failed to write file: {}", path.display()))?;
        
        Ok(ToolResult::success(format!(
            "Replaced {} occurrence(s) in {}",
            occurrences,
            path.display()
        )))
    }
}
```

### 3.5 Bash Tool

**File:** `catalyst-tools/src/bash.rs`

```rust
use super::*;
use anyhow::{bail, Context};
use std::process::{Command, Stdio};

pub struct BashTool;

impl Tool for BashTool {
    fn name(&self) -> &str {
        "bash"
    }
    
    fn description(&self) -> &str {
        "Execute a shell command. Use for git, npm, cargo, etc. Not for file operations."
    }
    
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The command to execute"
                },
                "timeout": {
                    "type": "integer",
                    "description": "Timeout in milliseconds",
                    "default": 120000
                }
            },
            "required": ["command"]
        })
    }
    
    fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult> {
        let command = args["command"].as_str()
            .context("command required")?;
        
        // Basic safety check
        let dangerous_patterns = ["rm -rf /", "sudo rm", "> /dev/sd"];
        for pattern in &dangerous_patterns {
            if command.contains(pattern) {
                bail!("Blocked dangerous command pattern: {}", pattern);
            }
        }
        
        // Use spawn_blocking for async safety
        let working_dir = ctx.working_dir.clone();
        let command = command.to_string();
        let timeout_ms = ctx.timeout_ms;
        
        let output = std::thread::spawn(move || {
            Command::new("bash")
                .arg("-c")
                .arg(&command)
                .current_dir(&working_dir)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
        }).join()
            .map_err(|_| anyhow::anyhow!("Command execution panicked"))?
            .context("Failed to execute command")?;
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        
        let result = if output.status.success() {
            stdout.to_string()
        } else {
            format!(
                "Exit code: {}\n{}\n{}",
                output.status.code().unwrap_or(-1),
                stderr,
                stdout
            )
        };
        
        Ok(ToolResult::success(result))
    }
}
```

### 3.6 Tool Registry

**File:** `catalyst-tools/src/registry.rs`

```rust
use super::*;
use std::collections::HashMap;

pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            tools: HashMap::new(),
        };
        
        registry.register(Box::new(ReadTool));
        registry.register(Box::new(WriteTool));
        registry.register(Box::new(EditTool));
        registry.register(Box::new(BashTool));
        
        registry
    }
    
    pub fn register(&mut self, tool: Box<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }
    
    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(|t| t.as_ref())
    }
    
    pub fn to_anthropic_tools(&self) -> Vec<serde_json::Value> {
        self.tools.values().map(|tool| {
            serde_json::json!({
                "name": tool.name(),
                "description": tool.description(),
                "input_schema": tool.parameters()
            })
        }).collect()
    }
    
    pub fn execute(&self, name: &str, args: serde_json::Value, ctx: &ToolContext) -> Result<ToolResult> {
        let tool = self.get(name)
            .with_context(|| format!("Unknown tool: {}", name))?;
        
        tool.execute(args, ctx)
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}
```

**Deliverable:** Working tool system with 4 tools

---

## Phase 4: Core Agent (Days 6-7)

### 4.1 Agent State

**File:** `catalyst-core/src/agent.rs`

```rust
use anyhow::Result;
use catalyst_llm::{AnthropicClient, Content, ContentBlock, Message, Role, StreamEvent, Tool};
use catalyst_tools::{ToolContext, ToolRegistry, ToolResult};
use futures::StreamExt;
use serde_json::Value;
use std::collections::HashMap;
use tokio::sync::mpsc;
use tracing::{debug, info};

pub struct Agent {
    client: AnthropicClient,
    tools: ToolRegistry,
    messages: Vec<Message>,
    system_prompt: String,
    working_dir: std::path::PathBuf,
}

pub enum AgentEvent {
    TextDelta { text: String },
    ThinkingDelta { thinking: String },
    ToolCall { id: String, name: String, args: Value },
    ToolResult { id: String, result: String, is_error: bool },
    Complete,
    Error(String),
}

impl Agent {
    pub fn new(client: AnthropicClient, tools: ToolRegistry, working_dir: std::path::PathBuf) -> Self {
        Self {
            client,
            tools,
            messages: Vec::new(),
            system_prompt: DEFAULT_SYSTEM_PROMPT.to_string(),
            working_dir,
        }
    }
    
    pub fn set_system_prompt(&mut self, prompt: String) {
        self.system_prompt = prompt;
    }
    
    pub async fn send(
        &mut self,
        user_message: String,
        tx: mpsc::UnboundedSender<AgentEvent>,
    ) -> Result<()> {
        // Add user message
        self.messages.push(Message {
            role: Role::User,
            content: Content::Text(user_message),
        });
        
        // Get anthropic tools
        let anthropic_tools = self.tools.to_anthropic_tools();
        
        // Stream response
        let mut stream = self.client.stream(
            Some(&self.system_prompt),
            self.messages.clone(),
            anthropic_tools,
        ).await?;
        
        let mut assistant_content: Vec<ContentBlock> = Vec::new();
        let mut current_text = String::new();
        let mut current_thinking = String::new();
        let mut current_tool_call: Option<(String, String, String)> = None;
        
        while let Some(event) = stream.next().await {
            match event? {
                StreamEvent::ContentBlockStart { content_block, .. } => {
                    match content_block {
                        ContentBlock::Text { .. } => {
                            current_text.clear();
                        }
                        ContentBlock::Thinking { .. } => {
                            current_thinking.clear();
                        }
                        ContentBlock::ToolUse { id, name, .. } => {
                            current_tool_call = Some((id, name, String::new()));
                        }
                        _ => {}
                    }
                }
                
                StreamEvent::ContentBlockDelta { delta, .. } => {
                    match delta {
                        catalyst_llm::Delta::TextDelta { text } => {
                            current_text.push_str(&text);
                            let _ = tx.send(AgentEvent::TextDelta { text });
                        }
                        catalyst_llm::Delta::ThinkingDelta { thinking } => {
                            current_thinking.push_str(&thinking);
                            let _ = tx.send(AgentEvent::ThinkingDelta { thinking });
                        }
                        catalyst_llm::Delta::InputJsonDelta { partial_json } => {
                            if let Some((_, _, ref mut args)) = current_tool_call {
                                args.push_str(&partial_json);
                            }
                        }
                    }
                }
                
                StreamEvent::ContentBlockStop { .. } => {
                    if !current_text.is_empty() {
                        assistant_content.push(ContentBlock::Text {
                            text: current_text.clone(),
                        });
                        current_text.clear();
                    }
                    
                    if !current_thinking.is_empty() {
                        assistant_content.push(ContentBlock::Thinking {
                            thinking: current_thinking.clone(),
                        });
                        current_thinking.clear();
                    }
                    
                    if let Some((id, name, args_json)) = current_tool_call.take() {
                        let args: Value = serde_json::from_str(&args_json)
                            .unwrap_or(Value::Object(serde_json::Map::new()));
                        
                        assistant_content.push(ContentBlock::ToolUse {
                            id: id.clone(),
                            name: name.clone(),
                            input: args.clone(),
                        });
                        
                        let _ = tx.send(AgentEvent::ToolCall {
                            id: id.clone(),
                            name: name.clone(),
                            args: args.clone(),
                        });
                        
                        // Execute tool in blocking context to avoid blocking async runtime
                        let tools = self.tools.clone();
                        let name_clone = name.clone();
                        let args_clone = args.clone();
                        let working_dir = self.working_dir.clone();
                        
                        let result = tokio::task::spawn_blocking(move || {
                            let ctx = ToolContext {
                                working_dir,
                                env: HashMap::new(),
                                timeout_ms: 120_000,
                            };
                            tools.execute(&name_clone, args_clone, &ctx)
                        }).await
                          .context("Tool execution panicked")?
                          .context("Tool execution failed")?;
                        
                        let (output, is_error) = match result {
                            Ok(r) => (r.output, false),
                            Err(e) => (e.to_string(), true),
                        };
                        
                        let _ = tx.send(AgentEvent::ToolResult {
                            id: id.clone(),
                            result: output.clone(),
                            is_error,
                        });
                        
                        // Add tool result to messages
                        self.messages.push(Message {
                            role: Role::Assistant,
                            content: Content::Blocks(assistant_content.clone()),
                        });
                        
                        self.messages.push(Message {
                            role: Role::User,
                            content: Content::Blocks(vec![ContentBlock::ToolResult {
                                tool_use_id: id,
                                content: output,
                                is_error,
                            }]),
                        });
                        
                        // Continue conversation with tool result
                        return self.continue_conversation(tx).await;
                    }
                }
                
                StreamEvent::MessageStop => {
                    if !assistant_content.is_empty() {
                        self.messages.push(Message {
                            role: Role::Assistant,
                            content: Content::Blocks(assistant_content),
                        });
                    }
                    let _ = tx.send(AgentEvent::Complete);
                }
                
                StreamEvent::Error { error } => {
                    let _ = tx.send(AgentEvent::Error(error.message));
                }
                
                _ => {}
            }
        }
        
        Ok(())
    }
    
    async fn continue_conversation(
        &mut self,
        tx: mpsc::UnboundedSender<AgentEvent>,
    ) -> Result<()> {
        // Get anthropic tools
        let anthropic_tools = self.tools.to_anthropic_tools();
        
        // Stream response (no user message - continuing from tool result)
        let mut stream = self.client.stream(
            Some(&self.system_prompt),
            self.messages.clone(),
            anthropic_tools,
        ).await?;
        
        let mut assistant_content: Vec<ContentBlock> = Vec::new();
        let mut current_text = String::new();
        let mut current_thinking = String::new();
        let mut current_tool_call: Option<(String, String, String)> = None;
        
        while let Some(event) = stream.next().await {
            match event? {
                StreamEvent::ContentBlockStart { content_block, .. } => {
                    match content_block {
                        ContentBlock::Text { .. } => {
                            current_text.clear();
                        }
                        ContentBlock::Thinking { .. } => {
                            current_thinking.clear();
                        }
                        ContentBlock::ToolUse { id, name, .. } => {
                            current_tool_call = Some((id, name, String::new()));
                        }
                        _ => {}
                    }
                }
                
                StreamEvent::ContentBlockDelta { delta, .. } => {
                    match delta {
                        catalyst_llm::Delta::TextDelta { text } => {
                            current_text.push_str(&text);
                            let _ = tx.send(AgentEvent::TextDelta { text });
                        }
                        catalyst_llm::Delta::ThinkingDelta { thinking } => {
                            current_thinking.push_str(&thinking);
                            let _ = tx.send(AgentEvent::ThinkingDelta { thinking });
                        }
                        catalyst_llm::Delta::InputJsonDelta { partial_json } => {
                            if let Some((_, _, ref mut args)) = current_tool_call {
                                args.push_str(&partial_json);
                            }
                        }
                    }
                }
                
                StreamEvent::ContentBlockStop { .. } => {
                    if !current_text.is_empty() {
                        assistant_content.push(ContentBlock::Text {
                            text: current_text.clone(),
                        });
                        current_text.clear();
                    }
                    
                    if !current_thinking.is_empty() {
                        assistant_content.push(ContentBlock::Thinking {
                            thinking: current_thinking.clone(),
                        });
                        current_thinking.clear();
                    }
                    
                    if let Some((id, name, args_json)) = current_tool_call.take() {
                        let args: Value = serde_json::from_str(&args_json)
                            .unwrap_or(Value::Object(serde_json::Map::new()));
                        
                        assistant_content.push(ContentBlock::ToolUse {
                            id: id.clone(),
                            name: name.clone(),
                            input: args.clone(),
                        });
                        
                        let _ = tx.send(AgentEvent::ToolCall {
                            id: id.clone(),
                            name: name.clone(),
                            args: args.clone(),
                        });
                        
                        // Execute tool in blocking context
                        let tools = self.tools.clone();
                        let name_clone = name.clone();
                        let args_clone = args.clone();
                        let working_dir = self.working_dir.clone();
                        
                        let result = tokio::task::spawn_blocking(move || {
                            let ctx = ToolContext {
                                working_dir,
                                env: HashMap::new(),
                                timeout_ms: 120_000,
                            };
                            tools.execute(&name_clone, args_clone, &ctx)
                        }).await
                          .context("Tool execution panicked")?
                          .context("Tool execution failed")?;
                        
                        let (output, is_error) = (result.output, false);
                        
                        let _ = tx.send(AgentEvent::ToolResult {
                            id: id.clone(),
                            result: output.clone(),
                            is_error,
                        });
                        
                        // Add tool result to messages
                        self.messages.push(Message {
                            role: Role::Assistant,
                            content: Content::Blocks(assistant_content.clone()),
                        });
                        
                        self.messages.push(Message {
                            role: Role::User,
                            content: Content::Blocks(vec![ContentBlock::ToolResult {
                                tool_use_id: id,
                                content: output,
                                is_error,
                            }]),
                        });
                        
                        // Recursively continue if more tool calls needed
                        return Box::pin(self.continue_conversation(tx)).await;
                    }
                }
                
                StreamEvent::MessageStop => {
                    if !assistant_content.is_empty() {
                        self.messages.push(Message {
                            role: Role::Assistant,
                            content: Content::Blocks(assistant_content),
                        });
                    }
                    let _ = tx.send(AgentEvent::Complete);
                }
                
                StreamEvent::Error { error } => {
                    let _ = tx.send(AgentEvent::Error(error.message));
                }
                
                _ => {}
            }
        }
        
        Ok(())
    }
}

const DEFAULT_SYSTEM_PROMPT: &str = r#"
You are Catalyst, a research-driven AI coding agent.

Your philosophy:
- Research best practices before making changes
- Explain WHY you make each choice
- Challenge user assumptions when wrong
- Prioritize correctness over speed
- Write stable, secure, flawless code

When editing code:
1. Read the relevant files first
2. Understand the context
3. Make minimal, focused changes
4. Verify your changes work

Available tools:
- read: Read file contents
- write: Create new files
- edit: Edit existing files
- bash: Execute shell commands

Always think through problems carefully before acting.
"#;
```

### 4.2 Module Exports

**File:** `catalyst-core/src/lib.rs`

```rust
mod agent;

pub use agent::*;

pub type Result<T> = anyhow::Result<T>;
```

**Deliverable:** Working agent with streaming

---

## Phase 5: TUI (Days 8-10)

### 5.1 App State

**File:** `catalyst-tui/src/app.rs`

```rust
use catalyst_core::AgentEvent;
use std::collections::HashMap;

pub struct App {
    pub messages: Vec<Message>,
    pub input: String,
    pub input_mode: InputMode,
    pub cursor_position: usize,
    pub scroll_offset: usize,
    pub model: String,
    pub tokens_used: u64,
    pub cost: f64,
    pub is_streaming: bool,
    pub should_quit: bool,
    pub pending_input: Option<String>,
}

pub enum InputMode {
    Normal,
    Insert,
}

pub enum Message {
    User { content: String },
    Assistant { content: String, thinking: Option<String> },
    ToolCall { id: String, name: String, status: ToolStatus },
    ToolResult { id: String, output: String, is_error: bool },
}

pub enum ToolStatus {
    Pending,
    Running,
    Complete,
    Failed,
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
            tokens_used: 0,
            cost: 0.0,
            is_streaming: false,
            should_quit: false,
            pending_input: None,
        }
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
            AgentEvent::ToolResult { id, result, is_error } => {
                if let Some(Message::ToolCall { status, .. }) = self.messages.last_mut() {
                    *status = if is_error { ToolStatus::Failed } else { ToolStatus::Complete };
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
            AgentEvent::Error(msg) => {
                self.messages.push(Message::Assistant {
                    content: format!("Error: {}", msg),
                    thinking: None,
                });
                self.is_streaming = false;
            }
        }
    }
}
```

### 5.2 UI Rendering

**File:** `catalyst-tui/src/ui.rs`

```rust
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use crate::app::{App, InputMode, Message};

pub fn ui(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(0),     // Messages
            Constraint::Length(3),  // Input
            Constraint::Length(1),  // Footer
        ])
        .split(frame.size());
    
    // Header
    let header = Paragraph::new(format!(
        "Catalyst | Model: {} | Tokens: {} | Cost: ${:.2}",
        app.model,
        app.tokens_used,
        app.cost
    ))
    .block(Block::default().borders(Borders::BOTTOM));
    frame.render_widget(header, chunks[0]);
    
    // Messages
    let items: Vec<ListItem> = app.messages
        .iter()
        .map(|msg| match msg {
            Message::User { content } => {
                ListItem::new(Line::from(vec![
                    Span::styled("You: ", Style::default().fg(Color::Cyan)),
                    Span::raw(content),
                ]))
            }
            Message::Assistant { content, thinking } => {
                let mut lines = vec![Line::from(vec![
                    Span::styled("Catalyst: ", Style::default().fg(Color::Green)),
                    Span::raw(content),
                ])];
                
                if let Some(t) = thinking {
                    lines.push(Line::from(vec![
                        Span::styled("Thinking: ", Style::default().fg(Color::Yellow)),
                        Span::raw(t),
                    ]));
                }
                
                ListItem::new(lines)
            }
            Message::ToolCall { name, status, .. } => {
                let status_text = match status {
                    crate::app::ToolStatus::Running => "⟳",
                    crate::app::ToolStatus::Complete => "✓",
                    crate::app::ToolStatus::Failed => "✗",
                    crate::app::ToolStatus::Pending => "⋯",
                };
                ListItem::new(Line::from(vec![
                    Span::styled(status_text, Style::default().fg(Color::Blue)),
                    Span::raw(" "),
                    Span::styled(name, Style::default().fg(Color::Magenta)),
                ]))
            }
            Message::ToolResult { output, is_error, .. } => {
                let color = if *is_error { Color::Red } else { Color::Gray };
                ListItem::new(Line::from(vec![
                    Span::styled("  → ", Style::default().fg(color)),
                    Span::styled(&output[..output.len().min(100)], Style::default().fg(color)),
                ]))
            }
        })
        .collect();
    
    let messages = List::new(items)
        .block(Block::default());
    frame.render_widget(messages, chunks[1]);
    
    // Input
    let input_style = match app.input_mode {
        InputMode::Normal => Style::default(),
        InputMode::Insert => Style::default().fg(Color::Yellow),
    };
    
    let input = Paragraph::new(app.input.as_str())
        .style(input_style)
        .block(Block::default()
            .borders(Borders::ALL)
            .title(match app.input_mode {
                InputMode::Normal => "Normal (i for insert)",
                InputMode::Insert => "Insert (Esc for normal)",
            }));
    frame.render_widget(input, chunks[2]);
    
    // Footer
    let footer = Paragraph::new("Ctrl+C: Quit | Enter: Send | /help: Commands")
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(footer, chunks[3]);
}
```

### 5.3 Event Loop

**File:** `catalyst-tui/src/lib.rs`

```rust
mod app;
mod ui;

pub use app::*;
pub use ui::*;

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{self, Stdout};
use std::time::Duration;
use tokio::sync::mpsc;

pub type Result<T> = anyhow::Result<T>;

pub struct TerminalGuard {
    stdout: Stdout,
}

impl TerminalGuard {
    pub fn new() -> Result<Self> {
        let mut stdout = io::stdout();
        enable_raw_mode()?;
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        Ok(Self { stdout })
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = execute!(self.stdout, LeaveAlternateScreen, DisableMouseCapture);
        let _ = disable_raw_mode();
    }
}

pub async fn run_app(app: &mut App, mut rx: mpsc::UnboundedReceiver<catalyst_core::AgentEvent>) -> Result<()> {
    let _guard = TerminalGuard::new()?;
    
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    
    loop {
        // Poll for input
        if event::poll(Duration::from_millis(16))? {
            match event::read()? {
                Event::Key(key) => {
                    handle_key(app, key);
                }
                _ => {}
            }
        }
        
        // Poll for agent events
        while let Ok(event) = rx.try_recv() {
            app.handle_event(event);
        }
        
        // Render
        terminal.draw(|frame| ui(frame, app))?;
        
        if app.should_quit {
            break;
        }
    }
    
    Ok(())
}

fn handle_key(app: &mut App, key: KeyEvent) {
    match app.input_mode {
        InputMode::Normal => {
            match key.code {
                KeyCode::Char('i') => {
                    app.input_mode = InputMode::Insert;
                }
                KeyCode::Char('q') | KeyCode::Char('c') if key.modifiers == KeyModifiers::CONTROL => {
                    app.should_quit = true;
                }
                _ => {}
            }
        }
        InputMode::Insert => {
            match key.code {
                KeyCode::Esc => {
                    app.input_mode = InputMode::Normal;
                }
                KeyCode::Enter => {
                    if !app.input.is_empty() {
                        app.pending_input = Some(app.input.clone());
                        app.input.clear();
                        app.cursor_position = 0;
                        app.is_streaming = true;
                    }
                }
                KeyCode::Char(c) => {
                    // UTF-8 safe: use char indices, not byte indices
                    let char_pos = app.input.chars().count();
                    if app.cursor_position >= char_pos {
                        app.input.push(c);
                    } else {
                        // Find byte position for character position
                        let byte_pos = app.input
                            .char_indices()
                            .nth(app.cursor_position)
                            .map(|(i, _)| i)
                            .unwrap_or(app.input.len());
                        app.input.insert(byte_pos, c);
                    }
                    app.cursor_position += 1;
                }
                KeyCode::Backspace => {
                    if app.cursor_position > 0 {
                        app.cursor_position -= 1;
                        // UTF-8 safe: find byte position for character position
                        let byte_pos = app.input
                            .char_indices()
                            .nth(app.cursor_position)
                            .map(|(i, _)| i)
                            .unwrap_or(app.input.len());
                        app.input.remove(byte_pos);
                    }
                }
                KeyCode::Left => {
                    if app.cursor_position > 0 {
                        app.cursor_position -= 1;
                    }
                }
                KeyCode::Right => {
                    let char_count = app.input.chars().count();
                    if app.cursor_position < char_count {
                        app.cursor_position += 1;
                    }
                }
                _ => {}
            }
        }
    }
}
```

**Deliverable:** Working TUI

---

## Phase 6: CLI Entry Point (Day 11)

### 6.1 Main

**File:** `catalyst-cli/src/main.rs`

```rust
use anyhow::Result;
use catalyst_core::Agent;
use catalyst_llm::AnthropicClient;
use catalyst_tools::ToolRegistry;
use catalyst_tui::{App, InputMode, run_app};
use clap::Parser;
use std::path::PathBuf;
use tokio::sync::mpsc;

#[derive(Parser)]
#[command(name = "catalyst")]
#[command(about = "A research-driven AI coding agent", long_about = None)]
struct Cli {
    /// Working directory
    #[arg(short, long, default_value = ".")]
    dir: PathBuf,
    
    /// Model to use
    #[arg(short, long, default_value = "claude-sonnet-4-20250514")]
    model: String,
    
    /// API key (or set ANTHROPIC_API_KEY env var)
    #[arg(long, env = "ANTHROPIC_API_KEY")]
    api_key: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    
    let cli = Cli::parse();
    
    let working_dir = cli.dir.canonicalize()?;
    
    // Create client
    let client = AnthropicClient::new(cli.api_key, cli.model.clone());
    
    // Create tool registry
    let tools = ToolRegistry::new();
    
    // Create agent
    let agent = Agent::new(client, tools, working_dir);
    
    // Create app
    let mut app = App::new(cli.model);
    
    // Create channel for agent events
    let (tx, rx) = mpsc::unbounded_channel();
    
    // Spawn agent task
    tokio::spawn(async move {
        // Agent loop handles incoming user messages
        // and sends events through tx
    });
    
    // Run TUI
    run_app(&mut app, rx).await?;
    
    Ok(())
}
```

**Deliverable:** Working CLI binary

---

## Phase 7: Polish & Testing (Days 12-14)

### 7.1 Add Slash Commands

- `/help` - Show help
- `/model` - Change model
- `/clear` - Clear messages
- `/exit` - Exit

### 7.2 Error Handling

- Graceful API error handling
- Retry logic for rate limits
- User-friendly error messages

### 7.3 Testing

```bash
cargo test --workspace
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

### 7.4 Documentation

- README.md
- USAGE.md
- CONTRIBUTING.md

---

## Build & Release

### Development Build

```bash
cargo build --workspace
cargo run --package catalyst-cli
```

### Release Build

```bash
cargo build --workspace --release
```

### Install Locally

```bash
cargo install --path catalyst-cli
```

---

## Success Criteria

| Criteria | Verification |
|----------|-------------|
| Compiles | `cargo build --workspace` succeeds |
| Tests pass | `cargo test --workspace` succeeds |
| Lints clean | `cargo clippy` no warnings |
| TUI renders | Run `catalyst` and see interface |
| Sends message | Type message, see response |
| Uses tools | Ask to read a file, see tool call |
| Streams | Response appears character by character |

---

## Timeline

| Day | Phase | Deliverable |
|-----|-------|-------------|
| 1 | Setup | Compiling workspace |
| 2-3 | LLM Client | Streaming API working |
| 4-5 | Tools | 4 tools working |
| 6-7 | Core Agent | Agent processes messages |
| 8-10 | TUI | Interactive interface |
| 11 | CLI | Binary runs |
| 12-14 | Polish | Ready for release |

---

---

## Next Steps After MVP

### ✅ Completed in v0.1.0-alpha
1. ~~Multi-provider support~~ (Added OpenRouter)
2. Basic TUI implementation
3. 4 core tools (read, write, edit, bash)
4. Slash commands
5. Configuration system

### 🎯 Next Release: v0.2.0-beta

---

## v0.2.0-beta Release Requirements

### Priority: Critical (Must Fix)

#### Bugs & Issues
1. **Error Handling Improvements**
   - [ ] Replace all `unwrap()` in production code with proper error handling
   - [ ] Add user-friendly error messages for common failures
   - [ ] Improve error recovery in agent stream processing
   - Files: `catalyst-cli/src/main.rs`, `catalyst-core/src/agent.rs`

2. **Edge Cases**
   - [ ] Handle empty messages gracefully
   - [ ] Handle network timeouts and retries
   - [ ] Handle malformed API responses
   - [ ] Handle rate limiting with exponential backoff
   - Files: `catalyst-llm/src/anthropic.rs`, `catalyst-llm/src/openrouter.rs`

3. **Configuration**
   - [ ] Add config file validation
   - [ ] Handle missing/corrupted config files
   - [ ] Add config migration for version updates
   - Files: `catalyst-cli/src/config.rs`

#### Security
1. **API Key Security**
   - [ ] Mask API keys in logs and error messages
   - [ ] Secure storage of API keys (keyring integration)
   - [ ] Environment variable sanitization
   - Files: `catalyst-cli/src/main.rs`, `catalyst-tui/src/app.rs`

2. **Command Execution**
   - [ ] Expand dangerous command patterns list
   - [ ] Add command timeout enforcement
   - [ ] Add command whitelist/blacklist configuration
   - Files: `catalyst-tools/src/tools.rs`

### Priority: High (Should Have)

#### Features
1. **Session Persistence**
   - [ ] Save conversation history to disk
   - [ ] Load previous sessions
   - [ ] Session management (list, delete, resume)
   - [ ] Auto-save on exit
   - New files: `catalyst-core/src/session.rs`, `catalyst-cli/src/session.rs`

2. **File References**
   - [ ] Support `@file` syntax in messages
   - [ ] Auto-read referenced files
   - [ ] Handle large files with streaming
   - Files: `catalyst-core/src/agent.rs`, `catalyst-tui/src/app.rs`

3. **Enhanced Tool System**
   - [ ] Add `glob` tool for file pattern matching
   - [ ] Add `grep` tool for content search
   - [ ] Add `list` tool for directory listing
   - [ ] Tool timeout configuration per tool
   - Files: `catalyst-tools/src/tools.rs`, `catalyst-tools/src/registry.rs`

4. **Streaming Improvements**
   - [ ] Show typing indicators
   - [ ] Cancel streaming responses
   - [ ] Resume interrupted streams
   - Files: `catalyst-core/src/agent.rs`, `catalyst-tui/src/lib.rs`

#### Testing
1. **Unit Tests**
   - [ ] Add tests for LLM client error handling
   - [ ] Add tests for tool execution edge cases
   - [ ] Add tests for configuration loading/saving
   - [ ] Add tests for session persistence
   - Target: 80% code coverage

2. **Integration Tests**
   - [ ] End-to-end conversation flow tests
   - [ ] Tool execution integration tests
   - [ ] TUI interaction tests
   - New directory: `tests/`

3. **Performance Tests**
   - [ ] Large file handling tests
   - [ ] Long conversation tests
   - [ ] Memory leak detection

### Priority: Medium (Nice to Have)

#### User Experience
1. **TUI Enhancements**
   - [ ] Add message search functionality
   - [ ] Add message copy/paste
   - [ ] Add conversation export (markdown, JSON)
   - [ ] Add syntax highlighting for code blocks
   - [ ] Add message timestamps
   - Files: `catalyst-tui/src/ui.rs`, `catalyst-tui/src/app.rs`

2. **Keyboard Shortcuts**
   - [ ] Add vim-style navigation
   - [ ] Add page up/down for message history
   - [ ] Add keyboard shortcuts for common actions
   - Files: `catalyst-tui/src/lib.rs`

3. **Themes**
   - [ ] Add multiple color themes
   - [ ] Add custom theme support
   - [ ] Add light/dark mode toggle
   - Files: `catalyst-tui/src/theme.rs`

#### Performance
1. **Optimizations**
   - [ ] Implement message pagination for long conversations
   - [ ] Add caching for frequently accessed files
   - [ ] Optimize TUI rendering for large outputs
   - Files: `catalyst-tui/src/app.rs`, `catalyst-tui/src/ui.rs`

2. **Memory Management**
   - [ ] Limit conversation history size
   - [ ] Implement message pruning strategies
   - [ ] Add memory usage monitoring

#### Documentation
1. **User Documentation**
   - [ ] Add troubleshooting guide
   - [ ] Add FAQ section
   - [ ] Add video tutorials
   - [ ] Add example configurations
   - Files: `USAGE.md`, `docs/`

2. **Developer Documentation**
   - [ ] Add architecture diagrams
   - [ ] Add API documentation
   - [ ] Add contribution workflow diagrams
   - Files: `CONTRIBUTING.md`, `docs/architecture.md`

### Priority: Low (Future Consideration)

#### Advanced Features
1. **Multi-file Operations**
   - [ ] Batch file operations
   - [ ] File watching and auto-reload
   - [ ] Directory tree visualization

2. **Code Analysis**
   - [ ] AST parsing for better code understanding
   - [ ] Symbol navigation
   - [ ] Code completion suggestions

3. **Integrations**
   - [ ] Git integration (commit, diff, blame)
   - [ ] IDE integration (LSP support)
   - [ ] CI/CD integration

4. **Advanced AI Features**
   - [ ] Multi-turn context management
   - [ ] Code explanation mode
   - [ ] Refactoring suggestions
   - [ ] Test generation

---

## Release Checklist v0.2.0-beta

### Pre-Release
- [ ] All critical bugs fixed
- [ ] All high-priority features implemented
- [ ] All tests passing
- [ ] Zero clippy warnings
- [ ] Code coverage > 70%
- [ ] Documentation updated
- [ ] CHANGELOG.md created
- [ ] Migration guide from v0.1.0 created

### Testing
- [ ] Manual testing on macOS
- [ ] Manual testing on Linux
- [ ] Manual testing on Windows
- [ ] Performance benchmarks
- [ ] Memory leak testing
- [ ] Edge case testing

### Documentation
- [ ] README.md updated
- [ ] USAGE.md updated
- [ ] CONTRIBUTING.md updated
- [ ] API documentation complete
- [ ] Examples added

### Release
- [ ] Version bumped in Cargo.toml
- [ ] Git tag created (v0.2.0-beta)
- [ ] GitHub release published
- [ ] Release notes written
- [ ] Binaries built for all platforms
- [ ] Homebrew formula updated (optional)
- [ ] Announcement posted

---

## Known Issues (v0.1.0-alpha)

### Critical
- None identified

### High
1. **API Error Handling**: Some API errors may cause panics instead of graceful degradation
2. **Large Files**: No streaming for very large file reads (>100MB)
3. **Network Issues**: No retry logic for transient network failures

### Medium
1. **Message History**: No pagination for very long conversations
2. **Syntax Highlighting**: Code blocks don't have syntax highlighting
3. **Session Persistence**: Conversations lost on exit
4. **File References**: No `@file` syntax support

### Low
1. **Themes**: Only one color theme available
2. **Export**: No conversation export functionality
3. **Search**: No message search functionality

---

## Technical Debt

### Code Quality
1. **Error Types**: Create custom error types instead of using `anyhow::Error` everywhere
2. **Logging**: Add structured logging with levels
3. **Metrics**: Add performance metrics collection
4. **Tracing**: Add distributed tracing support

### Architecture
1. **Modularity**: Better separation of concerns in agent module
2. **Testing**: More comprehensive test coverage
3. **Documentation**: More inline code documentation
4. **Type Safety**: Use newtypes for IDs and strong types

### Dependencies
1. **Audit**: Run `cargo audit` regularly
2. **Updates**: Keep dependencies up to date
3. **Minimization**: Remove unused dependencies

---

## Success Metrics for v0.2.0-beta

### Quality
- [ ] Zero critical bugs
- [ ] < 5 high-priority bugs
- [ ] Test coverage > 70%
- [ ] All clippy warnings resolved
- [ ] Zero security vulnerabilities

### Performance
- [ ] Startup time < 100ms
- [ ] Message send latency < 50ms
- [ ] Memory usage < 100MB idle
- [ ] Smooth 60fps UI rendering

### User Experience
- [ ] Error messages are helpful
- [ ] No unexpected crashes
- [ ] Intuitive keyboard navigation
- [ ] Clear documentation

---

## Timeline Estimate

| Phase | Duration | Tasks |
|-------|----------|-------|
| Bug Fixes | 1-2 weeks | Critical and high priority bugs |
| Session Persistence | 1 week | Save/load conversations |
| File References | 3-5 days | @file syntax support |
| Enhanced Tools | 1 week | glob, grep, list tools |
| Testing | 1 week | Comprehensive test suite |
| Documentation | 3-5 days | Update all docs |
| Polish | 3-5 days | UI improvements, performance |
| **Total** | **5-7 weeks** | **v0.2.0-beta release** |

---

## Future Roadmap

### v0.3.0
- TLA+ verification integration
- Simulation engine
- Advanced code analysis

### v0.4.0
- IDE integration (LSP)
- Git integration
- Multi-file operations

### v1.0.0
- Production-ready release
- Complete feature set
- Extensive testing
- Full documentation
- Multiple platform support
