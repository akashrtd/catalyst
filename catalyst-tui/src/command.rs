#[derive(Debug, Clone)]
pub enum Command {
    Help,
    Model { name: String },
    Clear,
    Exit,
    Config,
    Unknown(String),
}

impl Command {
    pub fn parse(input: &str) -> Option<Self> {
        let input = input.trim();
        if !input.starts_with('/') {
            return None;
        }

        let input = &input[1..];
        let parts: Vec<&str> = input.splitn(2, ' ').collect();
        let cmd = parts.first()?;

        Some(match *cmd {
            "help" | "h" | "?" => Command::Help,
            "model" | "m" => {
                let name = parts.get(1).map(|s| s.to_string()).unwrap_or_default();
                Command::Model { name }
            }
            "clear" | "c" => Command::Clear,
            "exit" | "quit" | "q" => Command::Exit,
            "config" | "cfg" => Command::Config,
            _ => Command::Unknown(input.to_string()),
        })
    }

    pub fn help_text() -> Vec<String> {
        vec![
            "┌─ Commands ─────────────────────────────┐".to_string(),
            "│ /help, /h, /?     Show this help      │".to_string(),
            "│ /model, /m <name> Switch model        │".to_string(),
            "│ /clear, /c        Clear conversation  │".to_string(),
            "│ /config, /cfg     Show config         │".to_string(),
            "│ /exit, /quit, /q  Exit Catalyst       │".to_string(),
            "└────────────────────────────────────────┘".to_string(),
        ]
    }
}

#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub name: String,
    pub display_name: String,
    pub cost_input: f64,
    pub cost_output: f64,
    pub provider: String,
}

#[derive(Debug, Clone)]
pub struct ProviderInfo {
    pub id: String,
    pub name: String,
    pub env_var: String,
    pub models: Vec<String>,
}

impl ProviderInfo {
    pub fn all() -> Vec<Self> {
        vec![
            Self {
                id: "anthropic".to_string(),
                name: "Anthropic".to_string(),
                env_var: "ANTHROPIC_API_KEY".to_string(),
                models: vec![
                    "claude-sonnet-4-20250514".to_string(),
                    "claude-opus-4-20250514".to_string(),
                    "claude-3-5-sonnet-20241022".to_string(),
                    "claude-3-5-haiku-20241022".to_string(),
                ],
            },
            Self {
                id: "openrouter".to_string(),
                name: "OpenRouter".to_string(),
                env_var: "OPENROUTER_API_KEY".to_string(),
                models: vec![
                    "anthropic/claude-sonnet-4".to_string(),
                    "anthropic/claude-opus-4".to_string(),
                    "anthropic/claude-3.5-sonnet".to_string(),
                    "openai/gpt-4o".to_string(),
                    "google/gemini-pro-1.5".to_string(),
                ],
            },
        ]
    }

    pub fn find(id: &str) -> Option<Self> {
        Self::all().into_iter().find(|p| p.id == id)
    }
}

impl ModelInfo {
    pub fn all() -> Vec<Self> {
        vec![
            Self {
                name: "claude-sonnet-4-20250514".to_string(),
                display_name: "Claude Sonnet 4".to_string(),
                cost_input: 3.0,
                cost_output: 15.0,
                provider: "anthropic".to_string(),
            },
            Self {
                name: "claude-opus-4-20250514".to_string(),
                display_name: "Claude Opus 4".to_string(),
                cost_input: 15.0,
                cost_output: 75.0,
                provider: "anthropic".to_string(),
            },
            Self {
                name: "claude-3-5-sonnet-20241022".to_string(),
                display_name: "Claude 3.5 Sonnet".to_string(),
                cost_input: 3.0,
                cost_output: 15.0,
                provider: "anthropic".to_string(),
            },
            Self {
                name: "claude-3-5-haiku-20241022".to_string(),
                display_name: "Claude 3.5 Haiku".to_string(),
                cost_input: 0.80,
                cost_output: 4.0,
                provider: "anthropic".to_string(),
            },
        ]
    }

    pub fn find(name: &str) -> Option<Self> {
        Self::all().into_iter().find(|m| {
            m.name == name
                || m.display_name.to_lowercase() == name.to_lowercase()
                || m.name.to_lowercase().contains(&name.to_lowercase())
        })
    }

    pub fn calculate_cost(&self, input_tokens: u64, output_tokens: u64) -> f64 {
        (input_tokens as f64 * self.cost_input / 1_000_000.0)
            + (output_tokens as f64 * self.cost_output / 1_000_000.0)
    }
}
