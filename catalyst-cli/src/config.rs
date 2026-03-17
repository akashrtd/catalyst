use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default)]
    pub system_prompt: Option<String>,
    #[serde(default)]
    pub working_dir: Option<PathBuf>,
}

fn default_model() -> String {
    "claude-sonnet-4-20250514".to_string()
}

fn default_max_tokens() -> u32 {
    4096
}

impl Default for Config {
    fn default() -> Self {
        Self {
            model: default_model(),
            api_key: None,
            provider: None,
            max_tokens: default_max_tokens(),
            system_prompt: None,
            working_dir: None,
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        if config_path.exists() {
            let contents = std::fs::read_to_string(&config_path)
                .with_context(|| format!("Failed to read config: {}", config_path.display()))?;
            let config: Config = toml::from_str(&contents)
                .with_context(|| format!("Failed to parse config: {}", config_path.display()))?;
            Ok(config)
        } else {
            Ok(Self::default())
        }
    }

    #[allow(dead_code)]
    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;

        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent).context("Failed to create config directory")?;
        }

        let contents = toml::to_string_pretty(self).context("Failed to serialize config")?;

        std::fs::write(&config_path, contents)
            .with_context(|| format!("Failed to write config: {}", config_path.display()))?;

        Ok(())
    }

    pub fn config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir().context("Could not find config directory")?;
        Ok(config_dir.join("catalyst").join("config.toml"))
    }

    pub fn merge_cli_args(
        &mut self,
        model: Option<String>,
        api_key: Option<String>,
        dir: Option<PathBuf>,
    ) {
        if let Some(m) = model {
            self.model = m;
        }
        if let Some(k) = api_key {
            self.api_key = Some(k);
        }
        if let Some(d) = dir {
            self.working_dir = Some(d);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.model, "claude-sonnet-4-20250514");
        assert_eq!(config.max_tokens, 4096);
        assert!(config.api_key.is_none());
        assert!(config.provider.is_none());
        assert!(config.system_prompt.is_none());
        assert!(config.working_dir.is_none());
    }

    #[test]
    fn test_config_serialization() {
        let config = Config {
            model: "claude-3-opus".to_string(),
            api_key: Some("test-key".to_string()),
            provider: Some("anthropic".to_string()),
            max_tokens: 8192,
            system_prompt: Some("Be helpful".to_string()),
            working_dir: Some(PathBuf::from("/tmp")),
        };

        let toml_str = toml::to_string_pretty(&config).unwrap();
        assert!(toml_str.contains("claude-3-opus"));
        assert!(toml_str.contains("test-key"));
        assert!(toml_str.contains("anthropic"));
        assert!(toml_str.contains("8192"));
    }

    #[test]
    fn test_config_deserialization() {
        let toml_str = r#"
model = "claude-3-haiku"
max_tokens = 2048
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.model, "claude-3-haiku");
        assert_eq!(config.max_tokens, 2048);
        assert!(config.api_key.is_none());
    }

    #[test]
    fn test_config_deserialization_with_all_fields() {
        let toml_str = r#"
model = "claude-3-opus"
api_key = "sk-test-123"
provider = "anthropic"
max_tokens = 8192
system_prompt = "You are helpful"
working_dir = "/home/user/projects"
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.model, "claude-3-opus");
        assert_eq!(config.api_key, Some("sk-test-123".to_string()));
        assert_eq!(config.provider, Some("anthropic".to_string()));
        assert_eq!(config.max_tokens, 8192);
        assert_eq!(config.system_prompt, Some("You are helpful".to_string()));
        assert_eq!(
            config.working_dir,
            Some(PathBuf::from("/home/user/projects"))
        );
    }

    #[test]
    fn test_merge_cli_args_model() {
        let mut config = Config::default();
        config.merge_cli_args(Some("claude-3-haiku".to_string()), None, None);
        assert_eq!(config.model, "claude-3-haiku");
    }

    #[test]
    fn test_merge_cli_args_api_key() {
        let mut config = Config::default();
        config.merge_cli_args(None, Some("new-key".to_string()), None);
        assert_eq!(config.api_key, Some("new-key".to_string()));
    }

    #[test]
    fn test_merge_cli_args_working_dir() {
        let mut config = Config::default();
        config.merge_cli_args(None, None, Some(PathBuf::from("/tmp/work")));
        assert_eq!(config.working_dir, Some(PathBuf::from("/tmp/work")));
    }

    #[test]
    fn test_merge_cli_args_override() {
        let mut config = Config {
            model: "old-model".to_string(),
            api_key: Some("old-key".to_string()),
            ..Default::default()
        };
        config.merge_cli_args(
            Some("new-model".to_string()),
            Some("new-key".to_string()),
            Some(PathBuf::from("/new/dir")),
        );
        assert_eq!(config.model, "new-model");
        assert_eq!(config.api_key, Some("new-key".to_string()));
        assert_eq!(config.working_dir, Some(PathBuf::from("/new/dir")));
    }

    #[test]
    fn test_merge_cli_args_none_values() {
        let mut config = Config {
            model: "existing-model".to_string(),
            api_key: Some("existing-key".to_string()),
            ..Default::default()
        };
        config.merge_cli_args(None, None, None);
        assert_eq!(config.model, "existing-model");
        assert_eq!(config.api_key, Some("existing-key".to_string()));
    }
}
