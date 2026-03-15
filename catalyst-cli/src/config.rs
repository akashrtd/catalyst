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
