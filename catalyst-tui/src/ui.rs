use crate::app::{App, InputMode, Message, PopupState, SystemLevel, ToolStatus};
use crate::command::ProviderInfo;
use crate::theme::{spinner_frame, Symbols, Theme};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

pub fn ui(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(frame.area());

    render_header(frame, app, chunks[0]);
    render_messages(frame, app, chunks[1]);
    render_input(frame, app, chunks[2]);
    render_footer(frame, app, chunks[3]);

    match &app.popup {
        PopupState::ProviderSelect { selected } => {
            render_provider_select_popup(frame, *selected);
        }
        PopupState::ApiKeyInput {
            provider_id,
            api_key_input,
        } => {
            render_api_key_popup(frame, provider_id, api_key_input);
        }
        PopupState::None => {}
    }
}

fn render_header(frame: &mut Frame, app: &App, area: Rect) {
    let outer_block = Block::default()
        .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
        .border_style(Style::default().fg(Theme::CYAN_DIM))
        .border_type(ratatui::widgets::BorderType::Rounded);

    let inner = outer_block.inner(area);
    frame.render_widget(outer_block, area);

    let streaming_indicator = if app.is_streaming {
        let spinner = spinner_frame(app.last_update.elapsed().as_millis() as usize / 100);
        vec![
            Span::raw(" "),
            Span::styled(
                spinner.to_string(),
                Style::default()
                    .fg(Theme::CYAN)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" {}", app.status_message),
                Style::default().fg(Theme::CYAN),
            ),
        ]
    } else {
        vec![]
    };

    let tokens_display = if app.input_tokens > 0 || app.output_tokens > 0 {
        format!(
            " {}k→ {}k← ",
            app.input_tokens / 1000,
            app.output_tokens / 1000
        )
    } else {
        String::new()
    };

    let cost_display = if app.cost > 0.0 {
        format!(" ${:.2}", app.cost)
    } else {
        String::new()
    };

    let mut header_spans = vec![
        Span::styled(
            "◈",
            Style::default()
                .fg(Theme::PURPLE)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " CATALYST",
            Style::default()
                .fg(Theme::CYAN)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled("╺─╸", Style::default().fg(Theme::CYAN_DIM)),
        Span::raw(" "),
        Span::styled(&app.model, Style::default().fg(Theme::PURPLE_DIM)),
        Span::styled(tokens_display, Style::default().fg(Color::Gray)),
        Span::styled(cost_display, Style::default().fg(Theme::AMBER_DIM)),
    ];
    header_spans.extend(streaming_indicator);

    let header = Paragraph::new(Line::from(header_spans));
    frame.render_widget(header, inner);
}

fn render_messages(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::LEFT | Borders::RIGHT)
        .border_style(Style::default().fg(Theme::BORDER))
        .border_type(ratatui::widgets::BorderType::Rounded);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let items: Vec<ListItem> = app
        .messages
        .iter()
        .flat_map(|msg| render_message(msg, app.last_update.elapsed().as_millis() as usize))
        .skip(app.scroll_offset)
        .take(inner.height as usize)
        .collect();

    let messages = List::new(items);
    frame.render_widget(messages, inner);
}

fn render_message(msg: &Message, tick: usize) -> Vec<ListItem<'static>> {
    match msg {
        Message::User { content } => {
            let lines: Vec<&str> = content.lines().collect();
            let mut items = Vec::new();
            let mut first = true;
            for line in lines {
                if first {
                    items.push(ListItem::new(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(Symbols::USER, Style::default().fg(Theme::CYAN)),
                        Span::raw(" "),
                        Span::styled(
                            "You",
                            Style::default()
                                .fg(Theme::CYAN)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(" ╾─", Style::default().fg(Theme::CYAN_DIM)),
                        Span::raw(" "),
                        Span::styled(line.to_string(), Style::default().fg(Theme::TEXT)),
                    ])));
                    first = false;
                } else {
                    items.push(ListItem::new(Line::from(vec![
                        Span::raw("      "),
                        Span::styled(line.to_string(), Style::default().fg(Theme::TEXT_DIM)),
                    ])));
                }
            }
            if items.is_empty() {
                items.push(ListItem::new(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(Symbols::USER, Style::default().fg(Theme::CYAN)),
                    Span::raw(" "),
                    Span::styled(
                        "You",
                        Style::default()
                            .fg(Theme::CYAN)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(" ╾─", Style::default().fg(Theme::CYAN_DIM)),
                ])));
            }
            items
        }

        Message::Assistant { content, thinking } => {
            let mut items = Vec::new();

            if let Some(t) = thinking {
                if !t.is_empty() {
                    for line in t.lines().take(3) {
                        items.push(ListItem::new(Line::from(vec![
                            Span::raw("  "),
                            Span::styled(Symbols::THINKING, Style::default().fg(Theme::PURPLE)),
                            Span::raw(" "),
                            Span::styled(
                                line.to_string(),
                                Style::default()
                                    .fg(Theme::PURPLE_DIM)
                                    .add_modifier(Modifier::ITALIC),
                            ),
                        ])));
                    }
                }
            }

            let lines: Vec<&str> = content.lines().collect();
            let mut first = true;
            for line in lines {
                if first {
                    items.push(ListItem::new(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(Symbols::ASSISTANT, Style::default().fg(Theme::PURPLE)),
                        Span::raw(" "),
                        Span::styled(
                            "Catalyst",
                            Style::default()
                                .fg(Theme::PURPLE)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(" ╾─", Style::default().fg(Theme::PURPLE_DIM)),
                        Span::raw(" "),
                        Span::styled(line.to_string(), Style::default().fg(Theme::TEXT)),
                    ])));
                    first = false;
                } else {
                    items.push(ListItem::new(Line::from(vec![
                        Span::raw("      "),
                        Span::styled(line.to_string(), Style::default().fg(Theme::TEXT_DIM)),
                    ])));
                }
            }

            if items.is_empty() {
                items.push(ListItem::new(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(Symbols::ASSISTANT, Style::default().fg(Theme::PURPLE)),
                    Span::raw(" "),
                    Span::styled(
                        "Catalyst",
                        Style::default()
                            .fg(Theme::PURPLE)
                            .add_modifier(Modifier::BOLD),
                    ),
                ])));
            }

            items
        }

        Message::ToolCall { name, status, .. } => {
            let (icon, color): (String, Color) = match status {
                ToolStatus::Pending => (Symbols::PENDING.to_string(), Color::Gray),
                ToolStatus::Running => {
                    let spinner = spinner_frame(tick / 100);
                    (spinner.to_string(), Theme::AMBER)
                }
                ToolStatus::Complete => (Symbols::CHECK.to_string(), Theme::GREEN),
                ToolStatus::Failed => (Symbols::CROSS.to_string(), Theme::RED),
            };

            vec![ListItem::new(Line::from(vec![
                Span::raw("    "),
                Span::styled(
                    icon,
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ),
                Span::raw(" "),
                Span::styled(Symbols::TOOL, Style::default().fg(Theme::CYAN_DIM)),
                Span::raw(" "),
                Span::styled(name.clone(), Style::default().fg(Theme::CYAN)),
            ]))]
        }

        Message::ToolResult {
            output, is_error, ..
        } => {
            let color = if *is_error {
                Theme::RED
            } else {
                Theme::TEXT_DIM
            };
            let icon = if *is_error {
                Symbols::CROSS
            } else {
                Symbols::RESULT
            };
            let preview: String = output.chars().take(150).collect();
            let preview = if preview.len() < output.len() {
                format!("{}…", preview)
            } else {
                preview
            };

            let preview_lines: Vec<String> =
                preview.lines().take(2).map(|s| s.to_string()).collect();
            let mut items = Vec::new();

            for (i, line) in preview_lines.iter().enumerate() {
                if i == 0 {
                    items.push(ListItem::new(Line::from(vec![
                        Span::raw("      "),
                        Span::styled(icon, Style::default().fg(color)),
                        Span::raw(" "),
                        Span::styled(line.clone(), Style::default().fg(color)),
                    ])));
                } else {
                    items.push(ListItem::new(Line::from(vec![
                        Span::raw("        "),
                        Span::styled(line.clone(), Style::default().fg(color)),
                    ])));
                }
            }

            if items.is_empty() {
                items.push(ListItem::new(Line::from(vec![
                    Span::raw("      "),
                    Span::styled(icon, Style::default().fg(color)),
                ])));
            }

            items
        }

        Message::System { content, level } => {
            let (icon, color) = match level {
                SystemLevel::Info => (Symbols::SYSTEM, Theme::CYAN_DIM),
                SystemLevel::Warning => ("⚠", Theme::AMBER),
                SystemLevel::Error => ("✕", Theme::RED),
            };

            vec![ListItem::new(Line::from(vec![
                Span::raw("  "),
                Span::styled(icon, Style::default().fg(color)),
                Span::raw(" "),
                Span::styled(content.clone(), Style::default().fg(color)),
            ]))]
        }
    }
}

fn render_input(frame: &mut Frame, app: &App, area: Rect) {
    let (border_color, title, title_color) = match app.input_mode {
        InputMode::Normal => (Theme::CYAN_DIM, " ◈ NORMAL ", Theme::CYAN_DIM),
        InputMode::Insert => (Theme::PURPLE, " ◈ INSERT ", Theme::PURPLE),
        InputMode::ProviderSelect | InputMode::ApiKeyInput => {
            (Theme::BORDER, " ◈ POPUP ", Theme::TEXT_DIM)
        }
    };

    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .border_type(ratatui::widgets::BorderType::Rounded)
        .title(title)
        .title_style(
            Style::default()
                .fg(title_color)
                .add_modifier(Modifier::BOLD),
        );

    let text_color = match app.input_mode {
        InputMode::Normal => Theme::TEXT_DIM,
        InputMode::Insert => Theme::TEXT,
        InputMode::ProviderSelect | InputMode::ApiKeyInput => Theme::TEXT_MUTED,
    };

    let input = Paragraph::new(app.input.as_str())
        .style(Style::default().fg(text_color))
        .block(input_block);

    frame.render_widget(input, area);

    if app.input_mode == InputMode::Insert {
        let cursor_x = area.x + 1 + app.cursor_position as u16;
        let cursor_y = area.y + 1;
        frame.set_cursor_position((cursor_x.min(area.x + area.width - 2), cursor_y));
    }
}

fn render_footer(frame: &mut Frame, _app: &App, area: Rect) {
    let divider = Span::styled(" │ ", Style::default().fg(Theme::BORDER));

    let footer = Paragraph::new(Line::from(vec![
        Span::styled(
            "i",
            Style::default()
                .fg(Theme::CYAN)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("nsert", Style::default().fg(Color::Gray)),
        divider.clone(),
        Span::styled(
            "Esc",
            Style::default()
                .fg(Theme::CYAN)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" normal", Style::default().fg(Color::Gray)),
        divider.clone(),
        Span::styled(
            "Enter",
            Style::default()
                .fg(Theme::CYAN)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" send", Style::default().fg(Color::Gray)),
        divider.clone(),
        Span::styled(
            "/help",
            Style::default()
                .fg(Theme::PURPLE)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" cmds", Style::default().fg(Color::Gray)),
        divider,
        Span::styled(
            "Ctrl+C",
            Style::default().fg(Theme::RED).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" quit", Style::default().fg(Color::Gray)),
    ]));
    frame.render_widget(footer, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn render_provider_select_popup(frame: &mut Frame, selected: usize) {
    let popup_area = centered_rect(50, 40, frame.area());

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Theme::PURPLE))
        .border_type(ratatui::widgets::BorderType::Rounded)
        .title(" Select Provider ")
        .title_style(
            Style::default()
                .fg(Theme::PURPLE)
                .add_modifier(Modifier::BOLD),
        );

    let providers = ProviderInfo::all();

    let items: Vec<ListItem> = providers
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let style = if i == selected {
                Style::default()
                    .fg(Theme::TEXT)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Theme::TEXT_DIM)
            };

            let prefix = if i == selected { "▸ " } else { "  " };
            ListItem::new(Line::from(vec![
                Span::styled(
                    prefix,
                    Style::default().fg(if i == selected {
                        Theme::CYAN
                    } else {
                        Theme::TEXT_DIM
                    }),
                ),
                Span::styled(&p.name, style),
            ]))
        })
        .collect();

    let list = List::new(items).block(block);
    frame.render_widget(list, popup_area);

    let help_text = Paragraph::new(Line::from(vec![
        Span::styled(
            "↑↓ ",
            Style::default()
                .fg(Theme::CYAN)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("navigate", Style::default().fg(Color::Gray)),
        Span::raw("  "),
        Span::styled(
            "Enter",
            Style::default()
                .fg(Theme::CYAN)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" select", Style::default().fg(Color::Gray)),
        Span::raw("  "),
        Span::styled(
            "Esc",
            Style::default()
                .fg(Theme::CYAN)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" cancel", Style::default().fg(Color::Gray)),
    ]))
    .alignment(Alignment::Center);

    let help_area = Rect::new(
        popup_area.x,
        popup_area.y + popup_area.height,
        popup_area.width,
        1,
    );
    frame.render_widget(help_text, help_area);
}

fn render_api_key_popup(frame: &mut Frame, provider_id: &str, api_key_input: &str) {
    let popup_area = centered_rect(60, 25, frame.area());

    frame.render_widget(Clear, popup_area);

    let provider = ProviderInfo::find(provider_id);
    let provider_name = provider
        .as_ref()
        .map(|p| p.name.as_str())
        .unwrap_or(provider_id);
    let env_var = provider
        .as_ref()
        .map(|p| p.env_var.as_str())
        .unwrap_or("API_KEY");

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Theme::PURPLE))
        .border_type(ratatui::widgets::BorderType::Rounded)
        .title(format!(" {} API Key ", provider_name))
        .title_style(
            Style::default()
                .fg(Theme::PURPLE)
                .add_modifier(Modifier::BOLD),
        );

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let content_area = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Length(3),
        ])
        .split(inner);

    let prompt = Paragraph::new(Line::from(vec![
        Span::styled(
            "Enter your API key for ",
            Style::default().fg(Theme::TEXT_DIM),
        ),
        Span::styled(
            provider_name,
            Style::default()
                .fg(Theme::CYAN)
                .add_modifier(Modifier::BOLD),
        ),
    ]))
    .alignment(Alignment::Center);
    frame.render_widget(prompt, content_area[0]);

    let env_hint = Paragraph::new(Line::from(vec![
        Span::styled("Or set ", Style::default().fg(Theme::TEXT_DIM)),
        Span::styled(env_var, Style::default().fg(Theme::AMBER)),
        Span::styled(" env var", Style::default().fg(Theme::TEXT_DIM)),
    ]))
    .alignment(Alignment::Center);
    frame.render_widget(env_hint, content_area[2]);

    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Theme::CYAN_DIM))
        .border_type(ratatui::widgets::BorderType::Rounded);

    let display_key: String = api_key_input.chars().map(|_| '•').collect();
    let input = Paragraph::new(display_key)
        .style(Style::default().fg(Theme::TEXT))
        .block(input_block);
    frame.render_widget(input, content_area[3]);

    let cursor_x = content_area[3].x + 1 + api_key_input.len() as u16;
    let cursor_y = content_area[3].y + 1;
    frame.set_cursor_position((
        cursor_x.min(content_area[3].x + content_area[3].width - 2),
        cursor_y,
    ));

    let help_text = Paragraph::new(Line::from(vec![
        Span::styled(
            "Enter",
            Style::default()
                .fg(Theme::CYAN)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" save", Style::default().fg(Color::Gray)),
        Span::raw("  "),
        Span::styled(
            "Esc",
            Style::default()
                .fg(Theme::CYAN)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" cancel", Style::default().fg(Color::Gray)),
    ]))
    .alignment(Alignment::Center);

    let help_area = Rect::new(
        popup_area.x,
        popup_area.y + popup_area.height,
        popup_area.width,
        1,
    );
    frame.render_widget(help_text, help_area);
}
