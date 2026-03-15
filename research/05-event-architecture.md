# Event Architecture

## Overview

Catalyst uses an async event-driven architecture to handle:
- User input (keyboard, mouse)
- LLM streaming responses
- Tool execution
- TUI rendering

## Core Components

```
┌─────────────────────────────────────────────────────────────┐
│                         EVENT LOOP                          │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐  │
│  │ Input Events │    │  LLM Stream  │    │ Tool Results │  │
│  │  (crossterm) │    │  (Anthropic) │    │  (Executor)  │  │
│  └──────┬───────┘    └──────┬───────┘    └──────┬───────┘  │
│         │                   │                   │          │
│         ▼                   ▼                   ▼          │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                   tokio::select!                     │   │
│  │                  (Event Router)                      │   │
│  └─────────────────────────┬───────────────────────────┘   │
│                            │                               │
│                            ▼                               │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                    App State                         │   │
│  │  - Messages                                          │   │
│  │  - Input buffer                                      │   │
│  │  - UI state                                          │   │
│  └─────────────────────────┬───────────────────────────┘   │
│                            │                               │
│                            ▼                               │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                   TUI Render                         │   │
│  │                  (ratatui)                           │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

## Event Types

```rust
pub enum Event {
    // Input events
    Key(KeyEvent),
    Mouse(MouseEvent),
    Resize { width: u16, height: u16 },
    Paste(String),
    
    // LLM events
    LlmStreamStart { message_id: String },
    LlmTextDelta { text: String },
    LlmToolCall { id: String, name: String, args: Value },
    LlmStreamEnd,
    LlmError(Error),
    
    // Tool events
    ToolStart { call_id: String, name: String },
    ToolOutput { call_id: String, output: String },
    ToolEnd { call_id: String, result: ToolResult },
    ToolError { call_id: String, error: ToolError },
    
    // App events
    Quit,
    MessageSent(String),
    StateChange(StateChange),
}

pub enum StateChange {
    InputModeChanged(InputMode),
    ModelChanged(String),
    ScrollPosition(usize),
}
```

## Channels

```rust
pub struct EventChannels {
    pub input_rx: UnboundedReceiver<Event>,
    pub llm_rx: UnboundedReceiver<Event>,
    pub tool_rx: UnboundedReceiver<Event>,
}

pub struct EventSenders {
    pub input_tx: UnboundedSender<Event>,
    pub llm_tx: UnboundedSender<Event>,
    pub tool_tx: UnboundedSender<Event>,
}
```

## Main Event Loop

```rust
pub async fn run(mut terminal: Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    let (senders, receivers) = create_channels();
    
    // Spawn input handler
    spawn_input_handler(senders.input_tx.clone());
    
    // Spawn LLM handler
    let llm_client = AnthropicClient::new(api_key);
    spawn_llm_handler(senders.llm_tx.clone(), llm_client);
    
    // Spawn tool executor
    let tool_registry = ToolRegistry::new();
    spawn_tool_executor(senders.tool_tx.clone(), tool_registry);
    
    // Main app state
    let mut app = App::new();
    
    loop {
        tokio::select! {
            // Input events
            Some(event) = receivers.input_rx.recv() => {
                if handle_input_event(&mut app, event) {
                    break;
                }
            }
            
            // LLM stream events
            Some(event) = receivers.llm_rx.recv() => {
                handle_llm_event(&mut app, event);
            }
            
            // Tool execution events
            Some(event) = receivers.tool_rx.recv() => {
                handle_tool_event(&mut app, event);
            }
            
            // Render at 60fps max
            _ = tokio::time::sleep(Duration::from_millis(16)) => {
                terminal.draw(|frame| ui(frame, &app))?;
            }
        }
    }
    
    Ok(())
}
```

## Input Handler

```rust
fn spawn_input_handler(tx: UnboundedSender<Event>) {
    tokio::spawn(async move {
        let mut reader = crossterm::event::EventStream::new();
        
        loop {
            match reader.next().await {
                Some(Ok(crossterm::event::Event::Key(key))) => {
                    let _ = tx.send(Event::Key(key));
                }
                Some(Ok(crossterm::event::Event::Mouse(mouse))) => {
                    let _ = tx.send(Event::Mouse(mouse));
                }
                Some(Ok(crossterm::event::Event::Resize(w, h))) => {
                    let _ = tx.send(Event::Resize { width: w, height: h });
                }
                _ => {}
            }
        }
    });
}
```

## LLM Handler

```rust
fn spawn_llm_handler(tx: UnboundedSender<Event>, client: AnthropicClient) {
    tokio::spawn(async move {
        // Listen for message send events
        // When received, stream LLM response
        loop {
            // Wait for message to send
            // Then stream response
            let stream = client.stream(messages, tools).await;
            
            let _ = tx.send(Event::LlmStreamStart { message_id: id });
            
            while let Some(event) = stream.next().await {
                match event {
                    StreamEvent::ContentDelta { delta: Delta::Text(text), .. } => {
                        let _ = tx.send(Event::LlmTextDelta { text });
                    }
                    StreamEvent::ContentBlockStart { block: ContentBlock::ToolUse { id, name, .. }, .. } => {
                        let _ = tx.send(Event::LlmToolCall { id, name, args: json!({}) });
                    }
                    _ => {}
                }
            }
            
            let _ = tx.send(Event::LlmStreamEnd);
        }
    });
}
```

## Tool Executor

```rust
fn spawn_tool_executor(tx: UnboundedSender<Event>, registry: ToolRegistry) {
    tokio::spawn(async move {
        // Listen for tool call events
        // Execute tools and send results back
        
        loop {
            // Wait for tool call request
            let (call_id, tool_name, args) = receive_tool_request().await;
            
            let _ = tx.send(Event::ToolStart { call_id: call_id.clone(), name: tool_name.clone() });
            
            let result = registry.get(&tool_name)
                .ok_or(ToolError::NotFound)
                .and_then(|tool| tool.execute(args, &ctx));
            
            match result {
                Ok(output) => {
                    let _ = tx.send(Event::ToolEnd { call_id, result: output });
                }
                Err(error) => {
                    let _ = tx.send(Event::ToolError { call_id, error });
                }
            }
        }
    });
}
```

## App State

```rust
pub struct App {
    // Messages
    pub messages: Vec<Message>,
    pub pending_tool_calls: HashMap<String, PendingToolCall>,
    
    // Input
    pub input: String,
    pub input_mode: InputMode,
    pub cursor_position: usize,
    
    // UI
    pub scroll_offset: usize,
    pub focused_pane: FocusedPane,
    
    // Status
    pub model: String,
    pub tokens_used: u64,
    pub cost: f64,
    pub is_streaming: bool,
    
    // Files
    pub working_dir: PathBuf,
    pub referenced_files: Vec<PathBuf>,
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

## Event Handling

```rust
fn handle_input_event(app: &mut App, event: Event) -> bool {
    match event {
        Event::Key(key) => match app.input_mode {
            InputMode::Normal => handle_normal_key(app, key),
            InputMode::Insert => handle_insert_key(app, key),
            InputMode::Command => handle_command_key(app, key),
        },
        Event::Resize { width, height } => {
            // Terminal resized, will auto-render
            false
        }
        _ => false,
    }
}

fn handle_insert_key(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
            false
        }
        KeyCode::Enter => {
            if !app.input.is_empty() {
                // Send message
                let content = app.input.clone();
                app.input.clear();
                app.messages.push(Message::User { content });
                // Trigger LLM request
            }
            false
        }
        KeyCode::Char(c) => {
            app.input.insert(app.cursor_position, c);
            app.cursor_position += 1;
            false
        }
        KeyCode::Backspace => {
            if app.cursor_position > 0 {
                app.cursor_position -= 1;
                app.input.remove(app.cursor_position);
            }
            false
        }
        _ => false,
    }
}
```

## Benefits

| Aspect | Benefit |
|--------|---------|
| **Responsiveness** | UI never blocks on LLM or tool execution |
| **Modularity** | Each component isolated, easy to test |
| **Scalability** | Easy to add new event sources |
| **Backpressure** | Channels handle overflow gracefully |
