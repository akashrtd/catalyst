# Testing Framework

## Requirements

- Unit tests for core logic
- Integration tests for LLM/tool interaction
- Mock LLM responses
- Performance benchmarks
- CI/CD compatible

## Testing Crates

### Built-in `#[test]` ⭐⭐⭐⭐⭐

Rust's built-in testing framework.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_message() {
        let json = r#"{"role": "user", "content": "Hello"}"#;
        let msg: Message = serde_json::from_str(json).unwrap();
        assert_eq!(msg.role, Role::User);
    }
}
```

### tokio::test ⭐⭐⭐⭐⭐

For async tests.

```rust
#[tokio::test]
async fn test_stream_response() {
    let client = MockLlmClient::new();
    let stream = client.stream("Hello").await;
    
    let events: Vec<_> = stream.collect().await;
    assert!(!events.is_empty());
}
```

### mockall ⭐⭐⭐⭐

Mock generation for traits.

```rust
use mockall::automock;

#[automock]
#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn send(&self, message: &str) -> Result<Response>;
}

#[tokio::test]
async fn test_with_mock() {
    let mut mock = MockLlmClient::new();
    mock.expect_send()
        .with(eq("Hello"))
        .times(1)
        .returning(|_| Ok(Response::default()));
    
    let agent = Agent::new(mock);
    agent.process("Hello").await.unwrap();
}
```

### wiremock ⭐⭐⭐⭐

HTTP mocking for API tests.

```rust
use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path};

#[tokio::test]
async fn test_anthropic_api() {
    let server = MockServer::start().await;
    
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "msg_123",
            "content": [{"type": "text", "text": "Hello!"}]
        })))
        .mount(&server)
        .await;
    
    let client = AnthropicClient::new_with_base(&server.uri(), "test-key");
    let response = client.send("Hi").await.unwrap();
    
    assert_eq!(response.content, "Hello!");
}
```

### proptest ⭐⭐⭐⭐

Property-based testing.

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_edit_replace(
        original in ".*",
        old in ".*",
        new in ".*"
    ) {
        // Test that edit doesn't panic
        let result = apply_edit(&original, &old, &new);
        // Invariant: result should be valid UTF-8
        prop_assert!(result.is_ok());
    }
}
```

### criterion ⭐⭐⭐⭐⭐

Benchmarking.

```rust
use criterion::{criterion_group, criterion_main, Criterion};

fn benchmark_parse(c: &mut Criterion) {
    let json = include_str!("../fixtures/large_response.json");
    
    c.bench_function("parse_large_response", |b| {
        b.iter(|| serde_json::from_str::<Response>(json))
    });
}

criterion_group!(benches, benchmark_parse);
criterion_main!(benches);
```

## Test Organization

```
catalyst/
├── catalyst-core/
│   ├── src/
│   │   ├── lib.rs
│   │   └── agent.rs
│   └── tests/              # Integration tests
│       ├── fixtures/       # Test data
│       │   ├── message.json
│       │   └── stream_events.txt
│       └── integration_test.rs
├── catalyst-llm/
│   ├── src/
│   └── tests/
│       ├── mock_server.rs
│       └── api_test.rs
└── catalyst-tools/
    ├── src/
    └── tests/
        └── tool_execution_test.rs
```

## Mock LLM Client

```rust
// catalyst-test-utils/src/mock_llm.rs
use std::collections::VecDeque;

pub struct MockLlmClient {
    responses: VecDeque<Response>,
}

impl MockLlmClient {
    pub fn new() -> Self {
        Self { responses: VecDeque::new() }
    }
    
    pub fn with_response(mut self, response: Response) -> Self {
        self.responses.push_back(response);
        self
    }
    
    pub fn with_text(mut self, text: &str) -> Self {
        self.responses.push_back(Response {
            content: text.to_string(),
            tool_calls: vec![],
        });
        self
    }
    
    pub fn with_tool_call(mut self, name: &str, args: Value) -> Self {
        self.responses.push_back(Response {
            content: String::new(),
            tool_calls: vec![ToolCall {
                id: format!("call_{}", uuid::Uuid::new_v4()),
                name: name.to_string(),
                args,
            }],
        });
        self
    }
}

#[async_trait]
impl LlmClient for MockLlmClient {
    async fn send(&mut self, _message: &str) -> Result<Response> {
        self.responses.pop_front()
            .ok_or_else(|| anyhow::anyhow!("No more mock responses"))
    }
}
```

## Test Fixtures

```rust
// tests/fixtures/mod.rs
use std::path::Path;

pub fn load_fixture(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    std::fs::read_to_string(path).unwrap()
}

pub fn sample_message() -> Message {
    Message {
        role: Role::User,
        content: Content::Text("Hello, Catalyst!".into()),
    }
}
```

## Integration Tests

```rust
// tests/integration_test.rs
use catalyst_test_utils::{MockLlmClient, MockToolRegistry};

#[tokio::test]
async fn test_full_conversation_flow() {
    // Setup
    let llm = MockLlmClient::new()
        .with_text("I'll help you with that.")
        .with_tool_call("read", json!({"path": "src/main.rs"}))
        .with_text("I found the issue!");
    
    let tools = MockToolRegistry::new()
        .with_tool("read", |args| {
            Ok(ToolResult::success("fn main() {}"))
        });
    
    let mut agent = Agent::new(llm, tools);
    
    // Execute
    let response = agent.send_message("Fix the bug").await.unwrap();
    
    // Verify
    assert!(response.content.contains("found the issue"));
    assert_eq!(agent.tool_calls().len(), 1);
}
```

## TUI Testing

Testing terminal UI is tricky. Options:

### 1. Snapshot Testing

```rust
use insta::assert_snapshot;

#[test]
fn test_render_message() {
    let app = App::with_message("Hello");
    let output = render_to_string(&app);
    assert_snapshot!(output);
}
```

### 2. Virtual Terminal

```rust
use vt100::Parser;

#[test]
fn test_tui_output() {
    let mut parser = Parser::new(24, 80);
    
    // Process terminal output
    parser.process(b"\x1b[2J\x1b[H"); // Clear screen
    
    let screen = parser.screen();
    assert_eq!(screen.rows(), 24);
}
```

### 3. Golden Tests

```rust
#[test]
fn test_help_screen() {
    let app = App::new();
    let help = app.render_help();
    
    // Compare with golden file
    let expected = include_str!("../fixtures/help_screen.txt");
    assert_eq!(help, expected);
}
```

## CI/CD Configuration

```yaml
# .github/workflows/test.yml
name: Test

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy, rustfmt
      
      - name: Cache cargo
        uses: actions/cache@v4
        with:
          path: ~/.cargo
          key: cargo-${{ hashFiles('**/Cargo.lock') }}
      
      - name: Check formatting
        run: cargo fmt --check
      
      - name: Clippy
        run: cargo clippy --all-targets -- -D warnings
      
      - name: Test
        run: cargo test --all
      
      - name: Doc tests
        run: cargo test --doc
```

## Cargo.toml

```toml
[dev-dependencies]
tokio = { version = "1", features = ["test-util"] }
mockall = "0.12"
wiremock = "0.5"
proptest = "1.4"
criterion = { version = "0.5", features = ["async_tokio"] }
insta = "1.34"
tempfile = "3.10"

[[bench]]
name = "parse"
harness = false
```
