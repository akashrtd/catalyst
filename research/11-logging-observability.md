# Logging & Observability

## Requirements

- Debug during development
- Troubleshoot user issues
- Performance monitoring
- Structured logs for analysis

## Crates

### tracing ⭐⭐⭐⭐⭐ (Recommended)

Modern, structured logging and diagnostics.

**Pros:**
- Structured logging (key-value pairs)
- Spans for tracing execution flow
- Async-aware
- Multiple subscribers (file, stdout, etc.)
- Integrates with tokio

**Cons:**
- Slightly more complex than log crate

```rust
use tracing::{info, debug, error, warn, instrument, span, Level};

#[instrument(skip(client))]
async fn send_message(client: &LlmClient, message: &str) -> Result<Response> {
    debug!(message_len = message.len(), "Sending message");
    
    let response = client.send(message).await?;
    
    info!(
        tokens_used = response.usage.total_tokens,
        model = %response.model,
        "Received response"
    );
    
    Ok(response)
}
```

### tracing-subscriber

Configuration for tracing.

```rust
use tracing_subscriber::{
    fmt, EnvFilter, layer::SubscriberExt, util::SubscriberInitExt,
};

fn init_logging() {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "catalyst=debug,tower_http=info".into()),
        )
        .init();
}
```

### tracing-appender

File logging.

```rust
use tracing_appender::{rolling, non_blocking::WorkerGuard};

fn init_file_logging() -> WorkerGuard {
    let file_appender = rolling::daily(
        dirs::data_dir().unwrap().join("catalyst/logs"),
        "catalyst.log"
    );
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    
    tracing_subscriber::registry()
        .with(fmt::layer().with_writer(non_blocking))
        .init();
    
    guard
}
```

## Logging Strategy

### Log Levels

| Level | Use Case |
|-------|----------|
| `ERROR` | Failures that stop operation |
| `WARN` | Unexpected but recoverable |
| `INFO` | Key events (session start, message sent) |
| `DEBUG` | Detailed flow (tool calls, API requests) |
| `TRACE` | Everything (full message content, raw JSON) |

### Structured Fields

```rust
// Good: Structured, queryable
info!(
    provider = "anthropic",
    model = "claude-sonnet-4-20250514",
    tokens.input = 150,
    tokens.output = 234,
    latency_ms = 1234,
    "LLM request completed"
);

// Bad: Unstructured
info!("LLM request completed with anthropic/claude-sonnet-4-20250514, tokens=384, latency=1234ms");
```

### Spans for Request Tracing

```rust
use tracing::{instrument, span, Level};

async fn handle_user_message(&mut self, message: String) -> Result<()> {
    // Create span for entire request
    let span = span!(Level::INFO, "user_request", message_len = message.len());
    let _enter = span.enter();
    
    // This will be nested under user_request
    let response = self.llm_client.send(&message).await?;
    
    // Tool calls will also nest
    for tool_call in response.tool_calls {
        self.execute_tool(&tool_call).await?;
    }
    
    Ok(())
}

#[instrument(skip(self, args))]
async fn execute_tool(&self, name: &str, args: &Value) -> Result<String> {
    debug!(tool = name, "Executing tool");
    let result = self.tools.execute(name, args)?;
    debug!(tool = name, result_len = result.len(), "Tool completed");
    Ok(result)
}
```

## Configuration

```rust
// catalyst-config/src/lib.rs
#[derive(Deserialize)]
pub struct LoggingConfig {
    #[serde(default = "default_level")]
    pub level: String,
    
    #[serde(default)]
    pub file: Option<String>,
    
    #[serde(default = "default_json_format")]
    pub json_format: bool,
}

fn default_level() -> String { "info".into() }
fn default_json_format() -> bool { false }
```

## Log Files

```
~/.local/share/catalyst/
├── logs/
│   ├── catalyst.2024-01-15.log
│   ├── catalyst.2024-01-16.log
│   └── catalyst.2024-01-17.log
└── sessions/
    ├── session-abc123.json
    └── session-def456.json
```

## Debug Mode

```rust
// catalyst-cli/src/main.rs
#[derive(Parser)]
struct Args {
    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,
    
    /// Enable trace logging (very verbose)
    #[arg(long)]
    trace: bool,
    
    /// Log to file
    #[arg(long)]
    log_file: Option<PathBuf>,
}

fn init_logging(args: &Args) {
    let level = if args.trace {
        "trace"
    } else if args.debug {
        "debug"
    } else {
        "info"
    };
    
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| format!("catalyst={}", level).into());
    
    // ... configure subscriber
}
```

## Performance Metrics

```rust
use std::time::Instant;

pub struct Metrics {
    pub llm_requests: u64,
    pub llm_tokens_input: u64,
    pub llm_tokens_output: u64,
    pub llm_latency_ms: u64,
    pub tool_executions: u64,
    pub tool_errors: u64,
}

impl Metrics {
    pub fn record_llm_request(&mut self, input: u64, output: u64, latency: Duration) {
        self.llm_requests += 1;
        self.llm_tokens_input += input;
        self.llm_tokens_output += output;
        self.llm_latency_ms += latency.as_millis() as u64;
        
        info!(
            requests = self.llm_requests,
            total_tokens = self.llm_tokens_input + self.llm_tokens_output,
            avg_latency_ms = self.llm_latency_ms / self.llm_requests,
            "LLM metrics"
        );
    }
}
```

## Error Context

```rust
use tracing::instrument;

#[instrument(skip(self), err)]
async fn risky_operation(&self) -> Result<()> {
    // err attribute automatically logs errors
    let data = self.fetch_data().await?;
    self.process_data(&data).await?;
    Ok(())
}
```

## Cargo.toml

```toml
[dependencies]
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
tracing-appender = "0.2"
```

## Debugging Tips

1. **Enable per-module logging:**
   ```bash
   RUST_LOG=catalyst=debug,catalyst_llm=trace catalyst
   ```

2. **JSON output for log analysis:**
   ```rust
   tracing_subscriber::fmt()
       .json()
       .with_target(false)
       .init();
   ```

3. **Correlation IDs:**
   ```rust
   let request_id = uuid::Uuid::new_v4();
   let span = span!(Level::INFO, "request", %request_id);
   ```
