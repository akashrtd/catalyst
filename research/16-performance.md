# Performance Optimization

## Goals

- Fast startup (< 100ms)
- Responsive TUI (60fps)
- Efficient LLM streaming
- Low memory footprint
- Minimal latency

## Startup Optimization

### Lazy Initialization

```rust
pub struct App {
    // Eagerly loaded
    config: Config,
    
    // Lazily loaded
    llm_client: OnceCell<LlmClient>,
    tool_registry: OnceCell<ToolRegistry>,
    theme: OnceCell<Theme>,
}

impl App {
    pub async fn llm_client(&self) -> &LlmClient {
        self.llm_client.get_or_init(|| async {
            LlmClient::new(&self.config.llm).await
        }).await
    }
}
```

### Feature Flags

```toml
# Cargo.toml
[features]
default = ["tui", "anthropic"]
tui = ["ratatui", "crossterm"]
anthropic = []
openai = []
simulation = ["docker"]
tla = ["tla-tools"]
```

### Compile-Time Optimization

```toml
# Cargo.toml (release profile)
[profile.release]
opt-level = 3
lto = true          # Link-time optimization
codegen-units = 1   # Better optimization, slower compile
strip = true        # Remove symbols
panic = "abort"     # Smaller binary
```

## Memory Optimization

### String Interning

```rust
use std::collections::HashSet;
use std::sync::Arc;

pub struct InternedString(Arc<str>);

impl InternedString {
    pub fn new(s: &str, cache: &mut HashSet<Arc<str>>) -> Self {
        let arc: Arc<str> = s.into();
        let interned = cache.get_or_insert(arc).clone();
        Self(interned)
    }
}
```

### Message Storage

```rust
// Store message content efficiently
pub enum MessageContent {
    Small(String),                          // < 256 bytes
    Large(Arc<str>),                        // >= 256 bytes, shared
    FileRef { path: Arc<Path>, range: Range<usize> },  // Reference file
}

pub struct Message {
    role: Role,
    content: MessageContent,
    timestamp: i64,  // Unix timestamp (8 bytes)
    // Total: ~32 bytes per message
}
```

### Arena Allocation

```rust
use bumpalo::Bump;

pub struct MessageArena {
    arena: Bump,
}

impl MessageArena {
    pub fn alloc_message(&self, msg: Message) -> &mut Message {
        self.arena.alloc(msg)
    }
    
    pub fn reset(&mut self) {
        self.arena.reset();
    }
}
```

## TUI Performance

### Differential Rendering

```rust
pub struct RenderCache {
    last_frame: Vec<BufferCell>,
    current_frame: Vec<BufferCell>,
}

impl RenderCache {
    pub fn render_diff(&mut self, frame: &mut Frame) {
        // Only update changed cells
        for (i, cell) in self.current_frame.iter().enumerate() {
            if self.last_frame.get(i) != Some(cell) {
                // Render only changed cell
                frame.render_widget(cell.widget(), cell.rect);
            }
        }
        
        std::mem::swap(&mut self.last_frame, &mut self.current_frame);
    }
}
```

### Rate-Limited Rendering

```rust
pub struct RenderThrottle {
    last_render: Instant,
    min_interval: Duration,  // ~16ms for 60fps
}

impl RenderThrottle {
    pub fn should_render(&mut self) -> bool {
        let now = Instant::now();
        if now.duration_since(self.last_render) >= self.min_interval {
            self.last_render = now;
            true
        } else {
            false
        }
    }
}
```

### Virtual Scrolling

```rust
pub struct VirtualList {
    items: Vec<Message>,
    visible_start: usize,
    visible_end: usize,
    item_height: u16,
    viewport_height: u16,
}

impl VirtualList {
    pub fn scroll_to(&mut self, offset: usize) {
        let visible_count = (self.viewport_height / self.item_height) as usize;
        self.visible_start = offset.min(self.items.len().saturating_sub(visible_count));
        self.visible_end = (self.visible_start + visible_count).min(self.items.len());
    }
    
    pub fn visible_items(&self) -> &[Message] {
        &self.items[self.visible_start..self.visible_end]
    }
}
```

## Streaming Performance

### Zero-Copy Parsing

```rust
use serde_json::de::from_slice;

pub fn parse_stream_event(data: &[u8]) -> Result<StreamEvent> {
    // Parse directly from bytes without copying
    from_slice(data).context("Failed to parse event")
}
```

### Buffer Pool

```rust
pub struct BufferPool {
    buffers: Vec<Vec<u8>>,
    buffer_size: usize,
}

impl BufferPool {
    pub fn get(&mut self) -> Vec<u8> {
        self.buffers.pop()
            .unwrap_or_else(|| vec![0u8; self.buffer_size])
    }
    
    pub fn return_buffer(&mut self, mut buf: Vec<u8>) {
        buf.clear();
        if self.buffers.len() < 16 {
            self.buffers.push(buf);
        }
    }
}
```

### Chunked Processing

```rust
pub struct StreamProcessor {
    buffer: Vec<u8>,
    chunk_size: usize,
}

impl StreamProcessor {
    pub fn process_chunk(&mut self, chunk: &[u8]) -> Vec<StreamEvent> {
        self.buffer.extend_from_slice(chunk);
        
        let mut events = Vec::new();
        
        while let Some(pos) = self.buffer.windows(2)
            .position(|w| w == b"\n\n") 
        {
            let event_data = self.buffer.drain(..pos).collect::<Vec<_>>();
            self.buffer.drain(..2);  // Remove \n\n
            
            if let Ok(event) = parse_stream_event(&event_data) {
                events.push(event);
            }
        }
        
        events
    }
}
```

## File Operations

### Async File I/O

```rust
use tokio::fs;

pub async fn read_file(path: &Path) -> Result<String> {
    // Use tokio async fs
    let content = fs::read_to_string(path).await?;
    Ok(content)
}

pub async fn read_file_chunk(path: &Path, offset: u64, len: u64) -> Result<Vec<u8>> {
    let file = fs::File::open(path).await?;
    let mut reader = tokio::io::BufReader::new(file);
    
    reader.seek(std::io::SeekFrom::Start(offset)).await?;
    
    let mut buffer = vec![0u8; len as usize];
    reader.read_exact(&mut buffer).await?;
    
    Ok(buffer)
}
```

### Caching

```rust
use lru::LruCache;
use std::num::NonZeroUsize;

pub struct FileCache {
    cache: LruCache<PathBuf, Arc<String>>,
}

impl FileCache {
    pub fn new() -> Self {
        Self {
            cache: LruCache::new(NonZeroUsize::new(100).unwrap()),
        }
    }
    
    pub async fn get(&mut self, path: &Path) -> Result<Arc<String>> {
        if let Some(content) = self.cache.get(path) {
            return Ok(content.clone());
        }
        
        let content = Arc::new(fs::read_to_string(path).await?);
        self.cache.put(path.to_path_buf(), content.clone());
        
        Ok(content)
    }
}
```

## LLM Optimization

### Prompt Caching (Anthropic)

```rust
pub struct CacheAwareRequest {
    system_prompt: String,
    tools: Vec<Tool>,
    messages: Vec<Message>,
}

impl CacheAwareRequest {
    pub fn build(&self) -> Request {
        Request {
            system: Some(SystemContent {
                type_: "text",
                text: &self.system_prompt,
                cache_control: Some(CacheControl { type_: "ephemeral" }),
            }),
            tools: self.tools.iter().map(|t| ToolDef {
                name: &t.name,
                // ...
                cache_control: Some(CacheControl { type_: "ephemeral" }),
            }).collect(),
            // ...
        }
    }
}
```

### Context Compaction

```rust
pub struct ContextCompactor;

impl ContextCompactor {
    pub fn compact(messages: &[Message], max_tokens: u64) -> Vec<Message> {
        let mut compacted = Vec::new();
        let mut token_count = 0;
        
        // Keep recent messages within limit
        for msg in messages.iter().rev() {
            let msg_tokens = estimate_tokens(&msg.content);
            
            if token_count + msg_tokens > max_tokens {
                break;
            }
            
            compacted.push(msg.clone());
            token_count += msg_tokens;
        }
        
        // Reverse to maintain order
        compacted.reverse();
        
        // Add summary of older messages
        if compacted.len() < messages.len() {
            let summary = summarize_messages(&messages[..messages.len() - compacted.len()]);
            compacted.insert(0, Message::system(summary));
        }
        
        compacted
    }
}

fn estimate_tokens(text: &str) -> u64 {
    // Rough estimate: ~4 characters per token
    (text.len() / 4) as u64
}
```

## Concurrency

### Parallel Tool Execution

```rust
pub async fn execute_tools_parallel(
    tools: &[ToolCall],
    registry: &ToolRegistry,
) -> Vec<ToolResult> {
    let futures: Vec<_> = tools.iter()
        .map(|call| registry.execute_async(&call.name, &call.args))
        .collect();
    
    futures::future::join_all(futures).await
}
```

### Connection Pooling

```rust
use reqwest::Client;

pub struct HttpClientPool {
    client: Client,
}

impl HttpClientPool {
    pub fn new() -> Self {
        let client = Client::builder()
            .pool_max_idle_per_host(10)
            .pool_idle_timeout(Duration::from_secs(30))
            .tcp_keepalive(Duration::from_secs(60))
            .build()
            .unwrap();
        
        Self { client }
    }
}
```

## Profiling

```rust
#[cfg(feature = "tracing")]
use tracing::instrument;

#[instrument(skip_all)]
pub async fn process_message(&mut self, message: &str) -> Result<Response> {
    let start = std::time::Instant::now();
    
    let response = self.llm_client.send(message).await?;
    
    tracing::info!(
        duration_ms = start.elapsed().as_millis() as u64,
        "Message processed"
    );
    
    Ok(response)
}
```

## Benchmarks

```rust
// benches/performance.rs
use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};

fn bench_parse_stream_event(c: &mut Criterion) {
    let data = include_bytes!("../fixtures/stream_event.json");
    
    c.bench_function("parse_stream_event", |b| {
        b.iter(|| parse_stream_event(data).unwrap())
    });
}

fn bench_render_message(c: &mut Criterion) {
    let mut group = c.benchmark_group("render");
    
    for size in [10, 100, 1000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let messages = generate_messages(size);
            b.iter(|| render_messages(&messages))
        });
    }
    
    group.finish();
}

criterion_group!(benches, bench_parse_stream_event, bench_render_message);
criterion_main!(benches);
```

## Performance Targets

| Metric | Target |
|--------|--------|
| Startup time | < 100ms |
| Time to first token | < 500ms |
| TUI frame rate | 60fps |
| Memory usage (idle) | < 50MB |
| Memory usage (active) | < 200MB |
| Binary size | < 10MB |

## Cargo.toml

```toml
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true
panic = "abort"

[dependencies]
lru = "0.12"
bumpalo = "3.15"

[dev-dependencies]
criterion = { version = "0.5", features = ["async_tokio"] }
```
