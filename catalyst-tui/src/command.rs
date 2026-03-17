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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_help_command() {
        assert!(matches!(Command::parse("/help"), Some(Command::Help)));
        assert!(matches!(Command::parse("/h"), Some(Command::Help)));
        assert!(matches!(Command::parse("/?"), Some(Command::Help)));
    }

    #[test]
    fn test_parse_clear_command() {
        assert!(matches!(Command::parse("/clear"), Some(Command::Clear)));
        assert!(matches!(Command::parse("/c"), Some(Command::Clear)));
    }

    #[test]
    fn test_parse_exit_command() {
        assert!(matches!(Command::parse("/exit"), Some(Command::Exit)));
        assert!(matches!(Command::parse("/quit"), Some(Command::Exit)));
        assert!(matches!(Command::parse("/q"), Some(Command::Exit)));
    }

    #[test]
    fn test_parse_config_command() {
        assert!(matches!(Command::parse("/config"), Some(Command::Config)));
        assert!(matches!(Command::parse("/cfg"), Some(Command::Config)));
    }

    #[test]
    fn test_parse_model_command() {
        match Command::parse("/model claude-3-opus") {
            Some(Command::Model { name }) => assert_eq!(name, "claude-3-opus"),
            _ => panic!("Expected Model command"),
        }

        match Command::parse("/m claude-3-haiku") {
            Some(Command::Model { name }) => assert_eq!(name, "claude-3-haiku"),
            _ => panic!("Expected Model command"),
        }
    }

    #[test]
    fn test_parse_model_command_empty() {
        match Command::parse("/model") {
            Some(Command::Model { name }) => assert!(name.is_empty()),
            _ => panic!("Expected Model command"),
        }
    }

    #[test]
    fn test_parse_unknown_command() {
        match Command::parse("/unknown") {
            Some(Command::Unknown(s)) => assert_eq!(s, "unknown"),
            _ => panic!("Expected Unknown command"),
        }
    }

    #[test]
    fn test_parse_non_command() {
        assert!(Command::parse("hello").is_none());
        assert!(Command::parse("not a command").is_none());
    }

    #[test]
    fn test_parse_with_whitespace() {
        assert!(matches!(Command::parse("  /help  "), Some(Command::Help)));
        assert!(matches!(Command::parse("/clear   "), Some(Command::Clear)));
    }

    #[test]
    fn test_model_info_find_exact() {
        let model = ModelInfo::find("claude-sonnet-4-20250514");
        assert!(model.is_some());
        let model = model.unwrap();
        assert_eq!(model.display_name, "Claude Sonnet 4");
        assert_eq!(model.provider, "anthropic");
    }

    #[test]
    fn test_model_info_find_by_display_name() {
        let model = ModelInfo::find("Claude Sonnet 4");
        assert!(model.is_some());
    }

    #[test]
    fn test_model_info_find_by_partial_name() {
        let model = ModelInfo::find("sonnet-4");
        assert!(model.is_some());
    }

    #[test]
    fn test_model_info_find_case_insensitive() {
        let model = ModelInfo::find("CLAUDE SONNET 4");
        assert!(model.is_some());
    }

    #[test]
    fn test_model_info_find_not_found() {
        let model = ModelInfo::find("nonexistent-model");
        assert!(model.is_none());
    }

    #[test]
    fn test_model_info_calculate_cost() {
        let model = ModelInfo::find("claude-sonnet-4-20250514").unwrap();

        let cost = model.calculate_cost(1_000_000, 1_000_000);
        assert!((cost - 18.0).abs() < 0.01);

        let cost = model.calculate_cost(500_000, 500_000);
        assert!((cost - 9.0).abs() < 0.01);

        let cost = model.calculate_cost(0, 0);
        assert!((cost - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_model_info_all() {
        let models = ModelInfo::all();
        assert!(!models.is_empty());
        assert!(models.iter().any(|m| m.name.contains("sonnet")));
        assert!(models.iter().any(|m| m.name.contains("opus")));
        assert!(models.iter().any(|m| m.name.contains("haiku")));
    }

    #[test]
    fn test_provider_info_all() {
        let providers = ProviderInfo::all();
        assert!(!providers.is_empty());
        assert!(providers.iter().any(|p| p.id == "anthropic"));
        assert!(providers.iter().any(|p| p.id == "openrouter"));
    }

    #[test]
    fn test_provider_info_find() {
        let provider = ProviderInfo::find("anthropic");
        assert!(provider.is_some());
        let provider = provider.unwrap();
        assert_eq!(provider.name, "Anthropic");
        assert_eq!(provider.env_var, "ANTHROPIC_API_KEY");
        assert!(!provider.models.is_empty());
    }

    #[test]
    fn test_provider_info_find_openrouter() {
        let provider = ProviderInfo::find("openrouter");
        assert!(provider.is_some());
        let provider = provider.unwrap();
        assert_eq!(provider.name, "OpenRouter");
        assert_eq!(provider.env_var, "OPENROUTER_API_KEY");
        assert!(!provider.models.is_empty());
    }

    #[test]
    fn test_provider_info_find_not_found() {
        let provider = ProviderInfo::find("nonexistent");
        assert!(provider.is_none());
    }

    #[test]
    fn test_help_text() {
        let help = Command::help_text();
        assert!(!help.is_empty());
        assert!(help.iter().any(|line| line.contains("help")));
        assert!(help.iter().any(|line| line.contains("model")));
        assert!(help.iter().any(|line| line.contains("clear")));
        assert!(help.iter().any(|line| line.contains("exit")));
    }
}
