# Security Considerations

## Threat Model

| Threat | Mitigation |
|--------|------------|
| API key exposure | Environment variables, file permissions |
| Malicious code execution | Tool restrictions, sandboxing |
| Path traversal | Path validation, working directory confinement |
| Prompt injection | Input sanitization, user confirmation |
| Data exfiltration | Network restrictions, audit logs |

## API Key Security

### Storage

```rust
// NEVER hardcode API keys
// BAD:
const API_KEY: &str = "sk-ant-...";  // NEVER DO THIS

// GOOD: Load from environment
fn get_api_key() -> Result<String> {
    env::var("ANTHROPIC_API_KEY")
        .context("ANTHROPIC_API_KEY not set")
}

// GOOD: Load from config with restricted permissions
fn load_api_key_from_config() -> Result<String> {
    let path = dirs::config_dir()
        .unwrap()
        .join("catalyst/api_key");
    
    // Check file permissions (Unix only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = fs::metadata(&path)?;
        let mode = metadata.permissions().mode();
        
        // File should be readable only by owner (0600)
        if mode & 0o077 != 0 {
            bail!("API key file has insecure permissions. Run: chmod 600 {:?}", path);
        }
    }
    
    let key = fs::read_to_string(&path)?.trim().to_string();
    Ok(key)
}
```

### Transmission

```rust
// Always use HTTPS
let client = reqwest::Client::builder()
    .https_only(true)  // Enforce HTTPS
    .build()?;

// Never log API keys
impl fmt::Debug for LlmClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LlmClient")
            .field("api_key", &"[REDACTED]")  // Never log actual key
            .finish()
    }
}
```

## Tool Execution Security

### Bash Tool Restrictions

```rust
pub struct BashTool {
    allowed_commands: HashSet<String>,
    blocked_patterns: Vec<Regex>,
    require_confirmation: bool,
}

impl BashTool {
    pub fn new(config: &BashToolConfig) -> Self {
        Self {
            allowed_commands: config.allowed_commands.iter().cloned().collect(),
            blocked_patterns: vec![
                Regex::new(r"rm\s+-rf\s+/").unwrap(),  // rm -rf /
                Regex::new(r">\s*/dev/sd").unwrap(),    // Overwrite disk
                Regex::new(r"curl.*\|\s*bash").unwrap(), // Curl pipe to bash
                Regex::new(r"wget.*\|\s*bash").unwrap(), // Wget pipe to bash
                Regex::new(r"sudo\s+").unwrap(),         // Sudo commands
                Regex::new(r"chmod\s+777").unwrap(),     // Insecure permissions
            ],
            require_confirmation: config.require_confirmation,
        }
    }
    
    fn validate_command(&self, command: &str) -> Result<()> {
        // Check blocked patterns
        for pattern in &self.blocked_patterns {
            if pattern.is_match(command) {
                bail!("Blocked dangerous pattern: {}", pattern);
            }
        }
        
        // Check allowed commands
        let cmd_name = command.split_whitespace().next()
            .context("Empty command")?;
        
        if !self.allowed_commands.contains(cmd_name) {
            bail!("Command '{}' is not in allowed list", cmd_name);
        }
        
        Ok(())
    }
}

impl Tool for BashTool {
    fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult> {
        let command = args["command"].as_str()
            .context("command required")?;
        
        self.validate_command(command)?;
        
        // Ask user confirmation for risky commands
        if self.require_confirmation && self.is_risky(command) {
            let confirmed = ctx.confirm("Execute this command?")?;
            if !confirmed {
                return Err(anyhow!("Command rejected by user"));
            }
        }
        
        // Execute with timeout
        let output = Command::new("bash")
            .arg("-c")
            .arg(command)
            .current_dir(&ctx.working_dir)
            .output()
            .context("Failed to execute command")?;
        
        // ... handle output
    }
    
    fn is_risky(&self, command: &str) -> bool {
        command.contains("rm ") || 
        command.contains("delete") ||
        command.contains("drop") ||
        command.contains("truncate")
    }
}
```

### Path Validation

```rust
pub fn validate_path(path: &Path, working_dir: &Path) -> Result<PathBuf> {
    // Canonicalize both paths
    let canonical_path = path.canonicalize()
        .context("Path does not exist")?;
    let canonical_working_dir = working_dir.canonicalize()
        .context("Working directory does not exist")?;
    
    // Ensure path is within working directory
    if !canonical_path.starts_with(&canonical_working_dir) {
        bail!(
            "Path '{}' is outside working directory '{}'",
            canonical_path.display(),
            canonical_working_dir.display()
        );
    }
    
    // Check for symlinks that escape working directory
    if canonical_path.is_symlink() {
        let target = canonical_path.read_link()
            .context("Failed to read symlink")?;
        validate_path(&target, working_dir)?;
    }
    
    Ok(canonical_path)
}

// Additional checks for read tool
pub fn validate_read_path(path: &Path, config: &ReadToolConfig) -> Result<()> {
    // Check file size
    let metadata = fs::metadata(path)?;
    if metadata.len() > config.max_file_size_bytes {
        bail!(
            "File too large: {} bytes (max: {})",
            metadata.len(),
            config.max_file_size_bytes
        );
    }
    
    // Check for sensitive files
    let filename = path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    
    let sensitive_patterns = [
        ".env",
        ".pem",
        ".key",
        "id_rsa",
        "credentials",
        "secrets",
        ".gitconfig",
    ];
    
    for pattern in &sensitive_patterns {
        if filename.contains(pattern) {
            bail!(
                "Refusing to read potentially sensitive file: {}",
                path.display()
            );
        }
    }
    
    Ok(())
}
```

## Input Validation

```rust
pub fn sanitize_user_input(input: &str) -> Result<String> {
    // Limit input length
    if input.len() > 100_000 {
        bail!("Input too long (max 100,000 characters)");
    }
    
    // Check for null bytes
    if input.contains('\0') {
        bail!("Input contains null bytes");
    }
    
    // Check for control characters (except newlines)
    for (i, ch) in input.char_indices() {
        if ch.is_control() && ch != '\n' && ch != '\t' {
            bail!("Input contains control character at position {}", i);
        }
    }
    
    Ok(input.to_string())
}

pub fn sanitize_path(path: &str) -> Result<String> {
    // No null bytes
    if path.contains('\0') {
        bail!("Path contains null bytes");
    }
    
    // No path traversal
    if path.contains("..") {
        bail!("Path contains '..' (path traversal attempt)");
    }
    
    // No absolute paths outside allowed directories
    if Path::new(path).is_absolute() {
        bail!("Absolute paths not allowed");
    }
    
    Ok(path.to_string())
}
```

## Network Security

```rust
// Restrict network access for tools
pub struct NetworkPolicy {
    allowed_hosts: HashSet<String>,
    blocked_hosts: HashSet<String>,
}

impl NetworkPolicy {
    pub fn check(&self, url: &str) -> Result<()> {
        let parsed = Url::parse(url)?;
        let host = parsed.host_str()
            .context("Invalid URL")?;
        
        if self.blocked_hosts.contains(host) {
            bail!("Host '{}' is blocked", host);
        }
        
        if !self.allowed_hosts.is_empty() && !self.allowed_hosts.contains(host) {
            bail!("Host '{}' is not in allowed list", host);
        }
        
        Ok(())
    }
}
```

## Audit Logging

```rust
use tracing::{info, warn};

pub struct AuditLogger;

impl AuditLogger {
    pub fn log_tool_execution(name: &str, args: &Value, result: &Result<ToolResult>) {
        match result {
            Ok(output) => {
                info!(
                    tool = name,
                    args = %args,
                    output_len = output.output.len(),
                    "Tool executed successfully"
                );
            }
            Err(e) => {
                warn!(
                    tool = name,
                    args = %args,
                    error = %e,
                    "Tool execution failed"
                );
            }
        }
    }
    
    pub fn log_llm_request(model: &str, tokens_input: u64) {
        info!(
            model = model,
            tokens_input = tokens_input,
            "LLM request sent"
        );
    }
    
    pub fn log_file_access(path: &Path, operation: &str) {
        info!(
            path = %path.display(),
            operation = operation,
            "File accessed"
        );
    }
}
```

## Dependency Security

```toml
# Cargo.toml - Use cargo-audit

[dev-dependencies]
cargo-audit = "0.21"

# Run regularly:
# cargo audit
```

```bash
# CI pipeline
- name: Security audit
  run: cargo audit
```

```bash
# CI pipeline
- name: Security audit
  run: cargo audit
```

## Secrets Detection

```rust
// Detect if code contains secrets
pub fn detect_secrets(content: &str) -> Vec<SecretMatch> {
    let patterns = [
        (r"sk-ant-[a-zA-Z0-9]{95}", "Anthropic API Key"),
        (r"sk-[a-zA-Z0-9]{20}T3Z[a-zA-Z0-9]{20}", "OpenAI API Key"),
        (r"ghp_[a-zA-Z0-9]{36}", "GitHub Personal Access Token"),
        (r"AKIA[0-9A-Z]{16}", "AWS Access Key ID"),
        (r"-----BEGIN (?:RSA |DSA |EC |OPENSSH )?PRIVATE KEY-----", "Private Key"),
    ];
    
    let mut matches = Vec::new();
    
    for (pattern, name) in &patterns {
        let re = Regex::new(pattern).unwrap();
        for capture in re.find_iter(content) {
            matches.push(SecretMatch {
                name: name.to_string(),
                start: capture.start(),
                end: capture.end(),
            });
        }
    }
    
    matches
}

pub fn check_for_secrets(content: &str) -> Result<()> {
    let secrets = detect_secrets(content);
    
    if !secrets.is_empty() {
        bail!(
            "Potential secrets detected:\n{}",
            secrets.iter()
                .map(|s| format!("  - {} at position {}", s.name, s.start))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
    
    Ok(())
}
```

## Security Checklist

- [ ] API keys loaded from environment or secure config
- [ ] Config files have restricted permissions (0600)
- [ ] Tool commands validated against allowlist
- [ ] Dangerous command patterns blocked
- [ ] Path traversal prevented
- [ ] File size limits enforced
- [ ] Sensitive files protected
- [ ] Input sanitization applied
- [ ] Network access restricted
- [ ] Audit logging enabled
- [ ] Dependencies audited
- [ ] Secrets detection in write operations

## Cargo.toml

```toml
[dependencies]
regex = "1.10"
url = "2.5"
sha2 = "0.10"  # For hashing secrets in logs
```
