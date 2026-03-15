use ratatui::style::Color;

pub struct Theme;

impl Theme {
    pub const BG_PRIMARY: Color = Color::Rgb(15, 15, 35);
    pub const BG_SECONDARY: Color = Color::Rgb(26, 27, 38);

    pub const CYAN: Color = Color::Rgb(0, 217, 255);
    pub const CYAN_DIM: Color = Color::Rgb(0, 140, 180);
    pub const PURPLE: Color = Color::Rgb(168, 85, 247);
    pub const PURPLE_DIM: Color = Color::Rgb(139, 92, 246);
    pub const MAGENTA: Color = Color::Rgb(255, 0, 255);
    pub const MAGENTA_DIM: Color = Color::Rgb(180, 0, 180);
    pub const AMBER: Color = Color::Rgb(251, 191, 36);
    pub const AMBER_DIM: Color = Color::Rgb(180, 130, 20);

    pub const GREEN: Color = Color::Rgb(34, 197, 94);
    pub const GREEN_DIM: Color = Color::Rgb(22, 163, 74);
    pub const RED: Color = Color::Rgb(239, 68, 68);
    pub const RED_DIM: Color = Color::Rgb(220, 38, 38);
    pub const BLUE: Color = Color::Rgb(59, 130, 246);
    pub const BLUE_DIM: Color = Color::Rgb(37, 99, 235);

    pub const TEXT: Color = Color::Rgb(230, 230, 240);
    pub const TEXT_DIM: Color = Color::Rgb(140, 140, 160);
    pub const TEXT_MUTED: Color = Color::Rgb(100, 100, 120);

    pub const BORDER: Color = Color::Rgb(60, 60, 90);
    pub const BORDER_ACCENT: Color = Color::Rgb(0, 217, 255);
    pub const BORDER_ACTIVE: Color = Color::Rgb(168, 85, 247);
}

pub struct Symbols;

impl Symbols {
    pub const ARROW: &str = "→";
    pub const DIVIDER: &str = "─";
    pub const DIVIDER_DOUBLE: &str = "═";
    pub const SECTION: &str = "◆";
    pub const USER: &str = "▸";
    pub const ASSISTANT: &str = "◆";
    pub const SYSTEM: &str = "◈";
    pub const TOOL: &str = "⚙";
    pub const RESULT: &str = "↳";
    pub const SPINNER: &str = "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏";
    pub const CHECK: &str = "✓";
    pub const CROSS: &str = "✗";
    pub const PENDING: &str = "⋯";
    pub const RUNNING: &str = "⟳";
    pub const BULLET: &str = "•";
    pub const THINKING: &str = "💭";
}

pub fn spinner_frame(frame: usize) -> char {
    let chars: Vec<char> = Symbols::SPINNER.chars().collect();
    chars[frame % chars.len()]
}
