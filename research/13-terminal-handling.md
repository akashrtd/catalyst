# Terminal Handling

## Requirements

- Cross-platform (Linux, macOS, Windows)
- Raw mode for interactive input
- Mouse support
- Resize handling
- Color support detection
- Alternate screen (for TUI)

## Crates

### crossterm ⭐⭐⭐⭐⭐ (Recommended)

Cross-platform terminal manipulation.

**Pros:**
- Works on Windows (unlike termion)
- Active maintenance
- Feature flags for minimal builds
- Event polling
- Supports raw mode, colors, cursor, screen

```toml
[dependencies]
crossterm = { version = "0.27", features = ["event-stream"] }
```

```rust
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};

fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    terminal::enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend)
}

fn restore_terminal() -> Result<()> {
    let mut stdout = stdout();
    execute!(stdout, LeaveAlternateScreen, DisableMouseCapture)?;
    terminal::disable_raw_mode()?;
    Ok(())
}
```

### termion ⭐⭐⭐⭐

Alternative terminal library (Unix only).

**Pros:**
- Simpler API
- Lightweight

**Cons:**
- No Windows support
- Less maintained

### termwiz ⭐⭐⭐

Feature-rich terminal library.

**Pros:**
- Built by WezTerm author
- Very capable

**Cons:**
- Larger dependency tree
- More complex

## Terminal Features

### Raw Mode

```rust
use crossterm::terminal;

// Enable raw mode (no line buffering, no echo)
terminal::enable_raw_mode()?;

// Your TUI loop here

// Restore
terminal::disable_raw_mode()?;
```

### Event Handling

```rust
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};

// Polling
if event::poll(Duration::from_millis(100))? {
    match event::read()? {
        Event::Key(key) => handle_key(key),
        Event::Mouse(mouse) => handle_mouse(mouse),
        Event::Resize(width, height) => handle_resize(width, height),
        _ => {}
    }
}

// Async with tokio
use crossterm::event::EventStream;
use futures::{StreamExt};

let mut reader = EventStream::new();

tokio::select! {
    Some(Ok(event)) = reader.next() => {
        match event {
            Event::Key(key) => handle_key(key),
            _ => {}
        }
    }
}
```

### Alternate Screen

```rust
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};

// Enter alternate screen (preserves shell history)
execute!(stdout, EnterAlternateScreen)?;

// Your TUI here

// Leave alternate screen (restores shell)
execute!(stdout, LeaveAlternateScreen)?;
```

### Mouse Support

```rust
use crossterm::event::{EnableMouseCapture, DisableMouseCapture, MouseEvent, MouseEventKind};

// Enable
execute!(stdout, EnableMouseCapture)?;

// Handle
match event {
    Event::Mouse(MouseEvent { kind, column, row, .. }) => {
        match kind {
            MouseEventKind::Down(button) => { /* click */ }
            MouseEventKind::ScrollUp => { /* scroll up */ }
            MouseEventKind::ScrollDown => { /* scroll down */ }
            _ => {}
        }
    }
}

// Disable
execute!(stdout, DisableMouseCapture)?;
```

### Colors & Styles

```rust
use crossterm::style::{Color, Print, SetForegroundColor, SetBackgroundColor, Attribute};

execute!(
    stdout,
    SetForegroundColor(Color::Green),
    SetBackgroundColor(Color::Black),
    Print("Success!"),
    SetAttribute(Attribute::Reset)
)?;
```

### Terminal Size

```rust
use crossterm::terminal;

let (width, height) = terminal::size()?;
println!("Terminal: {}x{}", width, height);
```

## Color Detection

```rust
use std::env;

fn supports_color() -> bool {
    // Check NO_COLOR
    if env::var("NO_COLOR").is_ok() {
        return false;
    }
    
    // Check TERM
    if let Ok(term) = env::var("TERM") {
        if term.contains("256color") || term.contains("truecolor") {
            return true;
        }
    }
    
    // Check COLORTERM
    if env::var("COLORTERM").is_ok() {
        return true;
    }
    
    false
}

fn color_depth() -> ColorDepth {
    if !supports_color() {
        return ColorDepth::Monochrome;
    }
    
    if let Ok(colorterm) = env::var("COLORTERM") {
        if colorterm == "truecolor" || colorterm == "24bit" {
            return ColorDepth::TrueColor;
        }
    }
    
    ColorDepth::Indexed256
}

enum ColorDepth {
    Monochrome,
    Indexed16,
    Indexed256,
    TrueColor,
}
```

## Signal Handling

```rust
use tokio::signal;

async fn setup_signals() {
    tokio::spawn(async {
        signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
        // Cleanup and exit
    });
    
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        
        tokio::spawn(async {
            let mut sigterm = signal(SignalKind::terminate()).unwrap();
            sigterm.recv().await;
            // Cleanup and exit
        });
        
        tokio::spawn(async {
            let mut sigwinch = signal(SignalKind::window_change()).unwrap();
            loop {
                sigwinch.recv().await;
                // Handle terminal resize
            }
        });
    }
}
```

## Terminal Setup Pattern

```rust
pub struct TerminalGuard {
    stdout: Stdout,
}

impl TerminalGuard {
    pub fn new() -> Result<Self> {
        let mut stdout = stdout();
        
        terminal::enable_raw_mode()?;
        execute!(
            stdout,
            EnterAlternateScreen,
            EnableMouseCapture
        )?;
        
        Ok(Self { stdout })
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = execute!(
            self.stdout,
            LeaveAlternateScreen,
            DisableMouseCapture
        );
        let _ = terminal::disable_raw_mode();
    }
}

// Usage
fn main() -> Result<()> {
    let _guard = TerminalGuard::new()?;
    
    // Terminal is set up
    // When _guard is dropped, terminal is restored
    
    run_app()?;
    Ok(())
}
```

## Panic Handling

```rust
use std::panic;

fn setup_panic_hook() {
    panic::set_hook(Box::new(|panic_info| {
        // Restore terminal first
        let _ = terminal::disable_raw_mode();
        let mut stdout = stdout();
        let _ = execute!(stdout, LeaveAlternateScreen, DisableMouseCapture);
        
        // Print panic
        eprintln!("\n{}", panic_info);
    }));
}
```

## Key Bindings

```rust
use crossterm::event::{KeyCode, KeyModifiers};

#[derive(Debug, Clone, Copy)]
pub enum Action {
    Quit,
    Submit,
    ScrollUp,
    ScrollDown,
    ChangeMode(InputMode),
    Help,
    None,
}

impl App {
    pub fn handle_key(&mut self, key: KeyEvent) -> Action {
        match (key.modifiers, key.code) {
            // Ctrl+C always quits
            (KeyModifiers::CONTROL, KeyCode::Char('c')) => Action::Quit,
            
            // Normal mode
            (KeyModifiers::NONE, KeyCode::Char('i')) if self.mode == Mode::Normal => {
                Action::ChangeMode(InputMode::Insert)
            }
            
            // Insert mode
            (KeyModifiers::NONE, KeyCode::Esc) if self.mode == Mode::Insert => {
                Action::ChangeMode(InputMode::Normal)
            }
            (KeyModifiers::NONE, KeyCode::Enter) if self.mode == Mode::Insert => {
                Action::Submit
            }
            
            // Navigation
            (KeyModifiers::NONE, KeyCode::Up) => Action::ScrollUp,
            (KeyModifiers::NONE, KeyCode::Down) => Action::ScrollDown,
            
            // Help
            (KeyModifiers::NONE, KeyCode::Char('?')) => Action::Help,
            
            _ => Action::None,
        }
    }
}
```

## Cargo.toml

```toml
[dependencies]
crossterm = { version = "0.27", features = ["event-stream"] }
tokio = { version = "1", features = ["signal"] }
```
