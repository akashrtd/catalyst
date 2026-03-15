# Async Runtime Comparison

## Requirements for Catalyst

- Handle concurrent I/O (LLM streaming, tool execution)
- Low latency for TUI responsiveness
- Good ecosystem integration
- Memory efficient

## Candidates

### tokio ⭐⭐⭐⭐⭐ (Recommended)

The de facto async runtime for Rust.

**Pros:**
- Most widely used (2.5B+ downloads)
- Excellent documentation
- Work-stealing scheduler (optimal CPU utilization)
- Full ecosystem: tokio::net, tokio::fs, tokio::sync, tokio::time
- Great integration with hyper (HTTP), tonic (gRPC)
- Active development, stable API
- `select!` macro for concurrent operations
- Channels: mpsc, oneshot, broadcast, watch

**Cons:**
- Larger binary size
- Slightly more complex than alternatives

**Best for Catalyst:**
```rust
use tokio::{select, sync::mpsc, time::{sleep, timeout}};

#[tokio::main]
async fn main() {
    let (tx, mut rx) = mpsc::channel(100);
    
    select! {
        Some(event) = rx.recv() => handle_event(event),
        _ = signal::ctrl_c() => println!("Interrupted"),
    }
}
```

### async-std ⭐⭐⭐

API mirrors standard library.

**Pros:**
- Familiar API (std::thread → async_std::task)
- Simpler mental model
- Good for beginners

**Cons:**
- Smaller ecosystem
- Fewer integrations
- Less battle-tested than tokio

### smol ⭐⭐⭐

Minimal, lightweight runtime.

**Pros:**
- Very small footprint
- Simple implementation
- Good for embedded

**Cons:**
- Small ecosystem
- Fewer features
- Not ideal for complex apps

## Comparison Matrix

| Criteria | tokio | async-std | smol |
|----------|-------|-----------|------|
| Ecosystem | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐ |
| Performance | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐⭐ |
| Documentation | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐ |
| Binary Size | Larger | Medium | Small |
| Learning Curve | Medium | Easy | Medium |
| Community | Largest | Medium | Small |

## tokio Feature Flags

```toml
[dependencies.tokio]
version = "1"
features = [
    "full",           # All features (development)
    # OR minimal for production:
    # "rt-multi-thread",  # Multi-threaded scheduler
    # "sync",             # Channels, mutex
    # "time",             # Sleep, timeout
    # "io-util",          # Async read/write traits
    # "fs",               # Filesystem operations
    # "process",          # Spawn processes (bash tool)
    # "signal",           # Ctrl+C handling
    # "macros",           # #[tokio::main], select!
]
```

## Channel Types

| Channel | Use Case |
|---------|----------|
| `mpsc` | Multiple producers, single consumer (events → main loop) |
| `oneshot` | Single value, single use (request → response) |
| `broadcast` | Multiple consumers (logging, events) |
| `watch` | Latest value only (state broadcasting) |

## Recommendation: tokio

For Catalyst's needs:
- Concurrent LLM streaming + tool execution + TUI rendering
- `tokio::select!` for handling multiple event sources
- Rich ecosystem for future features (HTTP server, etc.)
- Most resources/examples use tokio

## Usage Pattern

```rust
use tokio::{
    sync::mpsc::{self, UnboundedSender, UnboundedReceiver},
    time::{sleep, timeout, Duration},
};

pub struct EventLoop {
    input_rx: UnboundedReceiver<InputEvent>,
    llm_rx: UnboundedReceiver<LlmEvent>,
    tool_rx: UnboundedReceiver<ToolEvent>,
}

impl EventLoop {
    pub async fn run(mut self) {
        loop {
            tokio::select! {
                Some(event) = self.input_rx.recv() => {
                    self.handle_input(event);
                }
                Some(event) = self.llm_rx.recv() => {
                    self.handle_llm(event);
                }
                Some(event) = self.tool_rx.recv() => {
                    self.handle_tool(event);
                }
                _ = sleep(Duration::from_millis(16)) => {
                    self.render(); // ~60fps cap
                }
                _ = tokio::signal::ctrl_c() => {
                    break;
                }
            }
        }
    }
}
```
