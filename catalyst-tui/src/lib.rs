pub mod app;
pub mod command;
pub mod theme;
pub mod ui;

pub use app::{App, InputMode, Message, PopupState, SystemLevel, ToolStatus};
pub use command::{Command, ModelInfo, ProviderInfo};
pub use ui::ui;

use catalyst_core::AgentEvent;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers,
    },
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

pub async fn run_app(
    app: &mut App,
    mut rx: mpsc::UnboundedReceiver<AgentEvent>,
    tx: mpsc::UnboundedSender<String>,
    api_key_tx: mpsc::UnboundedSender<(String, String)>,
) -> Result<()> {
    let _guard = TerminalGuard::new()?;

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    loop {
        if event::poll(Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                match handle_key(app, key) {
                    HandleResult::Quit => break,
                    HandleResult::SendMessage(msg) => {
                        let _ = tx.send(msg);
                    }
                    HandleResult::UpdateApiKey { provider, api_key } => {
                        let _ = api_key_tx.send((provider, api_key));
                    }
                    HandleResult::Continue => {}
                }
            }
        }

        while let Ok(event) = rx.try_recv() {
            app.handle_event(event);
        }

        terminal.draw(|frame| ui(frame, app))?;

        if app.should_quit {
            break;
        }
    }

    Ok(())
}

pub enum HandleResult {
    Continue,
    Quit,
    SendMessage(String),
    UpdateApiKey { provider: String, api_key: String },
}

fn handle_key(app: &mut App, key: KeyEvent) -> HandleResult {
    match app.input_mode {
        InputMode::Normal => {
            match key.code {
                KeyCode::Char('i') => {
                    app.input_mode = InputMode::Insert;
                }
                KeyCode::Char('q') | KeyCode::Char('c')
                    if key.modifiers == KeyModifiers::CONTROL =>
                {
                    app.should_quit = true;
                    return HandleResult::Quit;
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if app.scroll_offset > 0 {
                        app.scroll_offset -= 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    app.scroll_offset = app.scroll_offset.saturating_add(1);
                }
                KeyCode::Char('/') => {
                    app.input_mode = InputMode::Insert;
                    app.input.push('/');
                }
                _ => {}
            }
            HandleResult::Continue
        }
        InputMode::Insert => match key.code {
            KeyCode::Esc => {
                app.input_mode = InputMode::Normal;
                HandleResult::Continue
            }
            KeyCode::Enter => {
                if !app.input.is_empty() {
                    let input = app.input.clone();

                    if let Some(cmd) = Command::parse(&input) {
                        match cmd {
                            Command::Help => {
                                app.show_help();
                            }
                            Command::Model { name } => {
                                if name.is_empty() {
                                    app.show_provider_select();
                                } else if let Some(model) = ModelInfo::find(&name) {
                                    app.set_model(model.name);
                                } else {
                                    app.add_system_message(
                                            format!("Unknown model: {}. Try 'claude-sonnet-4' or 'claude-opus-4'", name),
                                            SystemLevel::Warning,
                                        );
                                }
                            }
                            Command::Clear => {
                                app.clear_conversation();
                            }
                            Command::Exit => {
                                app.should_quit = true;
                                return HandleResult::Quit;
                            }
                            Command::Config => {
                                app.add_system_message(
                                    format!("Model: {}", app.model),
                                    SystemLevel::Info,
                                );
                                app.add_system_message(
                                    format!("Provider: {}", app.provider),
                                    SystemLevel::Info,
                                );
                                app.add_system_message(
                                    format!(
                                        "Tokens: {} in / {} out",
                                        app.input_tokens, app.output_tokens
                                    ),
                                    SystemLevel::Info,
                                );
                                app.add_system_message(
                                    format!("Cost: ${:.4}", app.cost),
                                    SystemLevel::Info,
                                );
                                let has_key = app.get_api_key(&app.provider).is_some();
                                app.add_system_message(
                                    format!(
                                        "API Key: {}",
                                        if has_key { "configured" } else { "not set" }
                                    ),
                                    SystemLevel::Info,
                                );
                            }
                            Command::Unknown(s) => {
                                app.add_system_message(
                                    format!("Unknown command: /{}. Type /help for commands.", s),
                                    SystemLevel::Warning,
                                );
                            }
                        }
                        app.input.clear();
                        app.cursor_position = 0;
                    } else {
                        app.messages.push(Message::User {
                            content: input.clone(),
                        });
                        app.input.clear();
                        app.cursor_position = 0;
                        app.is_streaming = true;
                        return HandleResult::SendMessage(input);
                    }
                }
                HandleResult::Continue
            }
            KeyCode::Char(c) => {
                let char_pos = app.input.chars().count();
                if app.cursor_position >= char_pos {
                    app.input.push(c);
                } else {
                    let byte_pos = app
                        .input
                        .char_indices()
                        .nth(app.cursor_position)
                        .map(|(i, _)| i)
                        .unwrap_or(app.input.len());
                    app.input.insert(byte_pos, c);
                }
                app.cursor_position += 1;
                HandleResult::Continue
            }
            KeyCode::Backspace => {
                if app.cursor_position > 0 {
                    app.cursor_position -= 1;
                    let byte_pos = app
                        .input
                        .char_indices()
                        .nth(app.cursor_position)
                        .map(|(i, _)| i)
                        .unwrap_or(app.input.len());
                    app.input.remove(byte_pos);
                }
                HandleResult::Continue
            }
            KeyCode::Left => {
                if app.cursor_position > 0 {
                    app.cursor_position -= 1;
                }
                HandleResult::Continue
            }
            KeyCode::Right => {
                let char_count = app.input.chars().count();
                if app.cursor_position < char_count {
                    app.cursor_position += 1;
                }
                HandleResult::Continue
            }
            _ => HandleResult::Continue,
        },
        InputMode::ProviderSelect => match key.code {
            KeyCode::Esc => {
                app.close_popup();
                HandleResult::Continue
            }
            KeyCode::Up => {
                if let PopupState::ProviderSelect { selected } = &mut app.popup {
                    if *selected > 0 {
                        *selected -= 1;
                    }
                }
                HandleResult::Continue
            }
            KeyCode::Down => {
                if let PopupState::ProviderSelect { selected } = &mut app.popup {
                    let providers = ProviderInfo::all();
                    if *selected < providers.len() - 1 {
                        *selected += 1;
                    }
                }
                HandleResult::Continue
            }
            KeyCode::Enter => {
                if let PopupState::ProviderSelect { selected } = &app.popup {
                    let providers = ProviderInfo::all();
                    if let Some(provider) = providers.get(*selected) {
                        let provider_id = provider.id.clone();
                        app.close_popup();
                        app.show_api_key_input(provider_id);
                    }
                }
                HandleResult::Continue
            }
            _ => HandleResult::Continue,
        },
        InputMode::ApiKeyInput => match key.code {
            KeyCode::Esc => {
                app.close_popup();
                HandleResult::Continue
            }
            KeyCode::Enter => {
                if let PopupState::ApiKeyInput {
                    provider_id,
                    api_key_input,
                } = &app.popup
                {
                    let provider = provider_id.clone();
                    let key = api_key_input.clone();

                    if !key.is_empty() {
                        app.api_keys.insert(provider.clone(), key.clone());
                        app.provider = provider.clone();

                        if let Some(provider_info) = ProviderInfo::find(&provider) {
                            if let Some(first_model) = provider_info.models.first() {
                                app.set_model(first_model.clone());
                            }
                        }

                        app.add_system_message(
                            format!("API key configured for {}", provider),
                            SystemLevel::Info,
                        );

                        app.close_popup();
                        return HandleResult::UpdateApiKey {
                            provider,
                            api_key: key,
                        };
                    }
                }
                app.close_popup();
                HandleResult::Continue
            }
            KeyCode::Char(c) => {
                if let PopupState::ApiKeyInput { api_key_input, .. } = &mut app.popup {
                    api_key_input.push(c);
                }
                HandleResult::Continue
            }
            KeyCode::Backspace => {
                if let PopupState::ApiKeyInput { api_key_input, .. } = &mut app.popup {
                    api_key_input.pop();
                }
                HandleResult::Continue
            }
            _ => HandleResult::Continue,
        },
    }
}
