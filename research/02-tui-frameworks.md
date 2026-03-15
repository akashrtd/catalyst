# TUI Frameworks for Rust

## Requirements

Catalyst's TUI needs:

1. **Responsive** - No lag during LLM streaming
2. **Composable** - Reusable components
3. **Event-driven** - Handle keyboard, mouse, resize
4. **Async-friendly** - Works with tokio
5. **Cross-platform** - Linux, macOS, Windows

## Candidates

### ratatui ⭐⭐⭐⭐⭐ (Recommended)

The de facto standard for Rust TUIs. Fork of the original tui-rs.

**Pros:**
- Mature, actively maintained
- Excellent documentation
- Widget-based architecture
- Built-in widgets: Paragraph, List, Table, Canvas, Chart
- Works with any terminal backend (crossterm, termion, termwiz)
- Large community, many examples
- Stateless rendering model (immediate mode)

**Cons:**
- Stateless model requires state management
- No built-in async integration (app must handle)

**Example:**
```rust
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};

use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers, MouseEventKind},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen, EnableMouseCapture, DisableMouseCapture},
};

use std::io::{self, stdout};

fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend)
}

fn restore_terminal() -> Result<()> {
    let mut stdout = stdout();
    execute!(stdout, LeaveAlternateScreen, DisableMouseCapture)?;
    disable_raw_mode()?;
    Ok(())
}
```

### cursive

Higher-level abstractions,Built-in views: dialogs, text inputs, lists
More "GUI-like" API

### makepad
GPU-accelerated rendering
Very fast
Modern architecture

Overkill for terminal app
Different target (native apps)

## Recommended Stack (March 2026)

| Crate | Version | Purpose |
|------|---------|---------|
| ratatui | 0.29+ | TUI framework |
| crossterm | 0.28+ | Terminal backend + events |

## Layout Structure

```
┌─────────────────────────────────────────────────────────────────┐
│ Header: Model, Context Usage, Cost                                  │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│ Messages:                                                       │
│   User: Fix the authentication bug                               │
│   Catalyst: I'll research the auth module...                        │
│   [Tool: read src/auth.rs]                                          │
│   [Result: 234 lines read]                                          │
│   Catalyst: Found the issue in verify_token...                           │
│                                                                 │
├─────────────────────────────────────────────────────────────────┤
│ Input: [Type your message...]                                       │
├─────────────────────────────────────────────────────────────────┤
│ Footer: /help for commands | Ctrl+C to exit                      │
└─────────────────────────────────────────────────────────────────┘
```

## Component Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                           TUI COMPONENTS                               │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ┌─────────┐   ┌─────────────┐   ┌───────────────────────────────┐   │
│  │ Header  │   │ MessageList │   │ Input (with @ mentions)   │   │
│  └─────────┘   └─────────────┘   └───────────────────────────────┘   │
│  - model    │   - messages    │   - cursor position              │   │
│  - tokens   │   - scroll      │   - file completions             │   │
│  - cost     │   - selection  │   - syntax highlighting            │   │
│                                                                     │
│  ┌─────────────────────┐   ┌─────────────────────────────────────┐   │
│  │ Message widgets      │   │ State widgets                       │   │
│  └─────────────────────┘   └─────────────────────────────────────┘   │
│                                                                     │
│  ┌──────────────┐   ┌───────────────┐   ┌──────────────┐   ┌────────────┐   │
│  │ UserMessage │   │ AssistantMsg │   │ ToolCall    │   │ ToolResult │   │
│  └──────────────┘   └───────────────┘   └──────────────┘   └────────────┘   │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

## State Management

```rust
pub struct App {
    // Messages
    messages: Vec<Message>,
    pending_tool_calls: HashMap<String, PendingToolCall>,
    
    // Input
    input: String,
    input_mode: InputMode,
    cursor_position: usize,
    
    // UI
    scroll_offset: usize,
    focused_pane: FocusedPane,
    
    // Status
    model: String,
    tokens_used: u64,
    cost: f64,
    is_streaming: bool,
    
    // Files
    working_dir: PathBuf,
    referenced_files: Vec<PathBuf>,
}

pub enum InputMode {
    Normal,
    Insert,
    Command,
}

pub enum FocusedPane {
    Messages,
    Input,
}

pub enum Message {
    User { content: String },
    Assistant { content: String, thinking: Option<String> },
    ToolCall { id: String, name: String, args: Value, status: ToolStatus },
    ToolResult { id: String, output: String, is_error: bool },
}

pub enum ToolStatus {
    Pending,
    Running,
    Complete,
    Failed,
}
```

## Event Flow with Tokio

```rust
use tokio::sync::mpsc;
use crossterm::event::EventStream;

pub async fn run_app() -> Result<()> {
    // Setup terminal
    let mut terminal = setup_terminal()?;
    
    // Create event channels
    let (input_tx, mut input_rx) = mpsc::unbounded_channel();
    let (llm_tx, mut llm_rx) = mpsc::unbounded_channel();
    let (tool_tx, mut tool_rx) = mpsc::unbounded_channel();
    
    // Spawn input handler
    tokio::spawn(async move {
        let mut reader = EventStream::new();
        while let Some(Ok(event)) = reader.next().await {
            if let Event::Key(key) = event {
                let _ = input_tx.send(Event::Key(key));
            }
        }
    });
    
    // Spawn LLM handler
    let llm_client = LlmClient::new(&config);
    tokio::spawn(async move {
        llm_client.handle_events(llm_tx).await;
    });
    
    // Spawn tool executor
    let tool_registry = ToolRegistry::new();
    tokio::spawn(async move {
        tool_registry.handle_events(tool_tx).await;
    });
    
    // Main event loop
    let mut app = App::new();
    
    loop {
        tokio::select! {
            Some(event) = input_rx.recv() => {
                app.handle_input_event(event);
            }
            Some(event) = llm_rx.recv() => {
                app.handle_llm_event(event);
            }
            Some(event) = tool_rx.recv() => {
                app.handle_tool_event(event);
            }
            _ = tokio::time::sleep(Duration::from_millis(16)) => {
                terminal.draw(|frame| ui(frame, &app))?;
            }
        }
    }
    
    Ok(())
}
```

## Rendering Strategy

```rust
impl App {
    pub fn render(&self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(3),
            ])
            .split(frame.size());
        
        // Header
        let header = self.render_header();
        frame.render_widget(header, chunks[0]);
        
        // Messages
        let messages = self.render_messages();
        frame.render_widget(messages, chunks[1]);
        
        // Input
        let input = self.render_input();
        frame.render_widget(input, chunks[2]);
        
        // Footer
        let footer = self.render_footer();
        frame.render_widget(footer, chunks[3]);
    }
}
```

## Key Bindings (MVP)

| Key | Mode | Action |
|-----|------|--------|
| `Ctrl+C` | Any | Exit |
| `Enter` | Insert | Send message |
| `Esc` | Any | Return to Normal mode |
| `i` | Normal | Enter Insert mode |
| `Up/Down` | Any | Scroll messages |
| `@` | Insert | Trigger file reference |
| `Ctrl+L` | Any | Change model |
| `Ctrl+H` | Any | Show help |
| `Tab` | Insert | Autocomplete |

## Resources
- [ratatui docs](https://docs.rs/ratatui)
- [ratatui examples](https://github.com/ratatui-org/ratatui/tree/main/examples)
- [crossterm docs](https://docs.rs/crossterm)
