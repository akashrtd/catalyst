# Error Handling Strategies

## Requirements

- User-friendly error messages (TUI display)
- Structured errors (programmatic handling)
- Error chaining (context propagation)
- Minimal boilerplate
- No panics in production

## Crates

### anyhow ⭐⭐⭐⭐⭐ (Application Errors)

For application-level error handling in the CLI.

**Pros:**
- Simple `Result<T, anyhow::Error>`
- `.context()` for adding context
- Automatic error conversion
- Backtraces (with feature flag)
- Great for CLI apps

**Cons:**
- Not for library code (too generic)
- Loses specific error types

```rust
use anyhow::{Context, Result};

fn read_config() -> Result<Config> {
    let path = "config.toml";
    let content = fs::read_to_string(path)
        .context(format!("Failed to read {}", path))?;
    let config: Config = toml::from_str(&content)
        .context("Failed to parse config")?;
    Ok(config)
}
```

### thiserror ⭐⭐⭐⭐⭐ (Library Errors)

For defining custom error types in libraries.

**Pros:**
- Derive macro for error enums
- Implements `std::error::Error`
- Full type information preserved
- Great for API boundaries

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LlmError {
    #[error("API request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),
    
    #[error("Rate limited, retry after {retry_after:?}")]
    RateLimited { retry_after: std::time::Duration },
    
    #[error("Invalid response: {message}")]
    InvalidResponse { message: String },
    
    #[error("Authentication failed")]
    AuthFailed,
}
```

### eyre ⭐⭐⭐⭐

Fork of anyhow with better reports.

**Pros:**
- Better error reports with `color-eyre`
- More customization
- Trait objects like anyhow

**Cons:**
- Slightly more complex setup

### snafu ⭐⭐⭐

Positional error creation.

**Pros:**
- Error context as struct fields
- Clear error sources

**Cons:**
- More verbose than thiserror

## Recommended Strategy

### Layered Approach

```
┌─────────────────────────────────────────────────────────┐
│  CLI Layer (catalyst-cli)                               │
│  Use: anyhow                                            │
│  Purpose: User-facing error messages, exit codes        │
└─────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────┐
│  Core Layer (catalyst-core, catalyst-llm, etc.)         │
│  Use: thiserror                                         │
│  Purpose: Typed errors, specific variants               │
└─────────────────────────────────────────────────────────┘
```

### Implementation

**Library crate (catalyst-llm):**
```rust
// catalyst-llm/src/error.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
    
    #[error("API error: {message} (code: {code})")]
    Api { code: u16, message: String },
    
    #[error("Stream error: {0}")]
    Stream(String),
    
    #[error("Rate limited, retry after {0:?}")]
    RateLimited(std::time::Duration),
    
    #[error("Context length exceeded: {used}/{limit}")]
    ContextExceeded { used: u64, limit: u64 },
}

pub type Result<T> = std::result::Result<T, Error>;
```

**Application crate (catalyst-cli):**
```rust
// catalyst-cli/src/main.rs
use anyhow::{Context, Result};
use catalyst_llm::Error as LlmError;

fn run() -> Result<()> {
    let response = client
        .send_message(prompt)
        .context("Failed to send message to LLM")?;
    
    Ok(())
}

fn main() {
    if let Err(err) = run() {
        eprintln!("Error: {}", err);
        std::process::exit(1);
    }
}
```

### Error Display in TUI

```rust
pub fn format_error_for_tui(err: &anyhow::Error) -> String {
    let mut output = String::new();
    
    // Main error
    output.push_str(&format!("❌ {}\n", err));
    
    // Chain of causes
    let mut cause = err.source();
    while let Some(err) = cause {
        output.push_str(&format!("  ↳ {}\n", err));
        cause = err.source();
    }
    
    // Suggestions (if available)
    if let Some(suggestion) = err.downcast_ref::<Suggestion>() {
        output.push_str(&format!("\n💡 {}\n", suggestion.0));
    }
    
    output
}
```

### Error Recovery

```rust
impl App {
    pub async fn handle_llm_error(&mut self, err: LlmError) -> Result<()> {
        match err {
            LlmError::RateLimited(duration) => {
                self.show_message(Message::system(format!(
                    "Rate limited. Retrying in {} seconds...",
                    duration.as_secs()
                )));
                tokio::time::sleep(duration).await;
                self.retry_last_request().await?;
            }
            LlmError::ContextExceeded { used, limit } => {
                self.show_message(Message::system(format!(
                    "Context exceeded: {}/{} tokens. Compacting...",
                    used, limit
                )));
                self.compact_context()?;
                self.retry_last_request().await?;
            }
            LlmError::Api { code: 401, .. } => {
                return Err(anyhow::anyhow!(
                    "Authentication failed. Check your API key."
                ));
            }
            err => {
                return Err(err.into());
            }
        }
        Ok(())
    }
}
```

## Best Practices

1. **Never panic** - Use `Result` everywhere
2. **Add context** - Use `.context()` liberally
3. **Preserve chain** - Don't lose original error
4. **User messages** - Convert to friendly text for TUI
5. **Recovery** - Handle retriable errors automatically
6. **Logging** - Log errors with full context

## Cargo.toml

```toml
[dependencies]
anyhow = "1.0"
thiserror = "1.0"
color-eyre = "0.6"  # Optional, for better panic reports
```
