# Configuration Management

## Requirements

- User configuration file (~/.config/catalyst/)
- Project-level configuration (./.catalyst/)
- Environment variables
- Sensible defaults
- Schema validation
- Hot reload (optional)

## Configuration Locations

```
~/.config/catalyst/           # User config (XDG)
    config.toml
    models.json
    themes/
        dark.toml
        light.toml

./.catalyst/                  # Project config
    config.toml
    agents.md                 # Project-specific instructions
    skills/                   # Project skills
```

## Configuration Structure

```rust
// catalyst-config/src/lib.rs
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub llm: LlmConfig,
    
    #[serde(default)]
    pub ui: UiConfig,
    
    #[serde(default)]
    pub tools: ToolConfig,
    
    #[serde(default)]
    pub logging: LoggingConfig,
}

#[derive(Debug, Deserialize)]
pub struct LlmConfig {
    #[serde(default = "default_provider")]
    pub provider: String,
    
    #[serde(default = "default_model")]
    pub model: String,
    
    #[serde(default)]
    pub api_key: Option<String>,
    
    #[serde(default = "default_api_key_env")]
    pub api_key_env: String,
    
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    
    #[serde(default)]
    pub temperature: Option<f32>,
    
    #[serde(default)]
    pub thinking_level: Option<ThinkingLevel>,
    
    #[serde(default)]
    pub cache: CacheConfig,
}

fn default_provider() -> String { "anthropic".into() }
fn default_model() -> String { "claude-sonnet-4-20250514".into() }
fn default_api_key_env() -> String { "ANTHROPIC_API_KEY".into() }
fn default_max_tokens() -> u32 { 4096 }

#[derive(Debug, Deserialize, Clone, Copy)]
pub enum ThinkingLevel {
    #[serde(rename = "off")]
    Off,
    #[serde(rename = "minimal")]
    Minimal,
    #[serde(rename = "low")]
    Low,
    #[serde(rename = "medium")]
    Medium,
    #[serde(rename = "high")]
    High,
}

#[derive(Debug, Deserialize, Default)]
pub struct CacheConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    
    #[serde(default)]
    pub system_prompt: bool,
    
    #[serde(default)]
    pub tools: bool,
}

#[derive(Debug, Deserialize)]
pub struct UiConfig {
    #[serde(default = "default_theme")]
    pub theme: String,
    
    #[serde(default = "default_true")]
    pub show_token_usage: bool,
    
    #[serde(default = "default_true")]
    pub show_cost: bool,
    
    #[serde(default)]
    pub show_thinking: bool,
    
    #[serde(default = "default_true")]
    pub mouse_support: bool,
}

fn default_theme() -> String { "dark".into() }

#[derive(Debug, Deserialize)]
pub struct ToolConfig {
    #[serde(default = "default_timeout")]
    pub default_timeout_ms: u64,
    
    #[serde(default)]
    pub bash: BashToolConfig,
    
    #[serde(default)]
    pub read: ReadToolConfig,
}

fn default_timeout() -> u64 { 120_000 }

#[derive(Debug, Deserialize)]
pub struct BashToolConfig {
    #[serde(default = "default_allowed_commands")]
    pub allowed_commands: Vec<String>,
    
    #[serde(default)]
    pub blocked_commands: Vec<String>,
}

fn default_allowed_commands() -> Vec<String> {
    vec![
        "git".into(),
        "cargo".into(),
        "npm".into(),
        "node".into(),
        "python".into(),
        "pip".into(),
        "make".into(),
    ]
}

#[derive(Debug, Deserialize)]
pub struct ReadToolConfig {
    #[serde(default = "default_max_file_size")]
    pub max_file_size_bytes: u64,
    
    #[serde(default = "default_max_lines")]
    pub max_lines: usize,
}

fn default_max_file_size() -> u64 { 10 * 1024 * 1024 } // 10MB
fn default_max_lines() -> usize { 2000 }

fn default_true() -> bool { true }
```

## Config Loading

```rust
impl Config {
    pub fn load() -> Result<Self> {
        let mut config = Config::default();
        
        // 1. Load user config
        if let Some(user_config) = Self::load_user_config()? {
            config = config.merge(user_config);
        }
        
        // 2. Load project config
        if let Some(project_config) = Self::load_project_config()? {
            config = config.merge(project_config);
        }
        
        // 3. Override with environment variables
        config = Self::apply_env_overrides(config);
        
        // 4. Resolve API key
        config = Self::resolve_api_key(config)?;
        
        Ok(config)
    }
    
    fn load_user_config() -> Result<Option<Self>> {
        let path = dirs::config_dir()
            .context("Could not find config directory")?
            .join("catalyst/config.toml");
        
        if !path.exists() {
            return Ok(None);
        }
        
        let content = fs::read_to_string(&path)
            .context("Failed to read config file")?;
        
        let config: Config = toml::from_str(&content)
            .context("Failed to parse config")?;
        
        Ok(Some(config))
    }
    
    fn load_project_config() -> Result<Option<Self>> {
        let path = PathBuf::from(".catalyst/config.toml");
        
        if !path.exists() {
            return Ok(None);
        }
        
        let content = fs::read_to_string(&path)?;
        let config: Config = toml::from_str(&content)?;
        
        Ok(Some(config))
    }
    
    fn apply_env_overrides(mut config: Self) -> Self {
        if let Ok(provider) = env::var("CATALYST_LLM_PROVIDER") {
            config.llm.provider = provider;
        }
        if let Ok(model) = env::var("CATALYST_LLM_MODEL") {
            config.llm.model = model;
        }
        if let Ok(key) = env::var("CATALYST_API_KEY") {
            config.llm.api_key = Some(key);
        }
        config
    }
    
    fn resolve_api_key(mut config: Self) -> Result<Self> {
        // If API key is directly set, use it
        if config.llm.api_key.is_some() {
            return Ok(config);
        }
        
        // Otherwise, read from environment variable
        let key = env::var(&config.llm.api_key_env)
            .context(format!(
                "API key not found. Set {} or configure api_key in config",
                config.llm.api_key_env
            ))?;
        
        config.llm.api_key = Some(key);
        Ok(config)
    }
    
    fn merge(self, other: Self) -> Self {
        // Merge logic: other overrides self
        Self {
            llm: LlmConfig {
                provider: other.llm.provider,
                model: other.llm.model,
                api_key: other.llm.api_key.or(self.llm.api_key),
                // ... etc
            },
            // ... other fields
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            llm: LlmConfig {
                provider: default_provider(),
                model: default_model(),
                api_key: None,
                api_key_env: default_api_key_env(),
                max_tokens: default_max_tokens(),
                temperature: None,
                thinking_level: None,
                cache: CacheConfig::default(),
            },
            ui: UiConfig {
                theme: default_theme(),
                show_token_usage: true,
                show_cost: true,
                show_thinking: false,
                mouse_support: true,
            },
            tools: ToolConfig {
                default_timeout_ms: default_timeout(),
                bash: BashToolConfig {
                    allowed_commands: default_allowed_commands(),
                    blocked_commands: vec![],
                },
                read: ReadToolConfig {
                    max_file_size_bytes: default_max_file_size(),
                    max_lines: default_max_lines(),
                },
            },
            logging: LoggingConfig::default(),
        }
    }
}
```

## Example Config File

```toml
# ~/.config/catalyst/config.toml

[llm]
provider = "anthropic"
model = "claude-sonnet-4-20250514"
# api_key = "sk-ant-..."  # Or set ANTHROPIC_API_KEY env var
max_tokens = 4096
temperature = 1.0

[llm.thinking]
level = "medium"

[llm.cache]
enabled = true
system_prompt = true
tools = true

[ui]
theme = "dark"
show_token_usage = true
show_cost = true
show_thinking = false
mouse_support = true

[tools]
default_timeout_ms = 120000

[tools.bash]
allowed_commands = ["git", "cargo", "npm", "node", "python"]
blocked_commands = ["rm -rf /", "sudo"]

[tools.read]
max_file_size_bytes = 10485760  # 10MB
max_lines = 2000

[logging]
level = "info"
# file = "~/.local/share/catalyst/logs/catalyst.log"
```

## Project Config (.catalyst/config.toml)

```toml
# ./.catalyst/config.toml

[llm]
model = "claude-opus-4-20250514"  # Use more powerful model for this project

[ui]
theme = "light"  # Project-specific theme

[tools.bash]
# Allow additional commands for this project
allowed_commands = ["git", "cargo", "npm", "docker", "kubectl"]
```

## AGENTS.md (Project Instructions)

```markdown
# Project Rules

## Build Commands
- Build: `cargo build`
- Test: `cargo test`
- Lint: `cargo clippy`

## Code Style
- Use `anyhow` for errors
- Document public APIs
- No unwrap in production code

## Architecture
- Follow hexagonal architecture
- Domain logic in `src/domain/`
- External adapters in `src/adapters/`
```

## Config Validation

```rust
impl Config {
    pub fn validate(&self) -> Result<()> {
        // Validate LLM config
        if self.llm.max_tokens == 0 {
            bail!("max_tokens must be > 0");
        }
        
        if let Some(temp) = self.llm.temperature {
            if !(0.0..=2.0).contains(&temp) {
                bail!("temperature must be between 0.0 and 2.0");
            }
        }
        
        // Validate tools config
        if self.tools.default_timeout_ms == 0 {
            bail!("default_timeout_ms must be > 0");
        }
        
        Ok(())
    }
}
```

## Config Generation

```rust
impl Config {
    pub fn generate_default() -> Result<()> {
        let config_dir = dirs::config_dir()
            .context("Could not find config directory")?
            .join("catalyst");
        
        fs::create_dir_all(&config_dir)?;
        
        let config_path = config_dir.join("config.toml");
        
        if config_path.exists() {
            bail!("Config file already exists at {:?}", config_path);
        }
        
        let default_config = r#"
# Catalyst Configuration
# See https://github.com/catalyst/catalyst for documentation

[llm]
provider = "anthropic"
model = "claude-sonnet-4-20250514"
max_tokens = 4096

[ui]
theme = "dark"
"#;
        
        fs::write(&config_path, default_config)?;
        
        println!("Created config at {:?}", config_path);
        Ok(())
    }
}
```

## Cargo.toml

```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
toml = "0.8"
dirs = "5.0"
```
