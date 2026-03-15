# Tool System Design

## MVP Tools

Catalyst starts with 4 core tools, similar to Pi:

| Tool | Description |
|------|-------------|
| `read` | Read file contents |
| `write` | Create new file |
| `edit` | Edit existing file |
| `bash` | Execute shell commands |

## Tool Interface

```rust
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> Value; // JSON Schema
    fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError>;
}

pub struct ToolContext {
    pub working_dir: PathBuf,
    pub env: HashMap<String, String>,
    pub timeout: Duration,
}

pub struct ToolResult {
    pub output: String,
    pub metadata: HashMap<String, Value>,
}

pub enum ToolError {
    InvalidArgs(String),
    ExecutionFailed(String),
    Timeout,
    PermissionDenied,
}
```

## Tool Definitions

### read

```rust
pub struct ReadTool;

impl Tool for ReadTool {
    fn name(&self) -> &str { "read" }
    
    fn description(&self) -> &str {
        "Read a file from the filesystem. Returns the file contents with line numbers."
    }
    
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The absolute or relative path to the file to read"
                },
                "offset": {
                    "type": "integer",
                    "description": "Line number to start reading from (1-indexed)",
                    "default": 1
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of lines to read",
                    "default": 2000
                }
            },
            "required": ["path"]
        })
    }
    
    fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let path = args["path"].as_str().ok_or(ToolError::InvalidArgs("path required".into()))?;
        let path = ctx.working_dir.join(path);
        
        let offset = args.get("offset").and_then(|v| v.as_u64()).unwrap_or(1) as usize;
        let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(2000) as usize;
        
        let content = fs::read_to_string(&path)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
        
        let lines: Vec<_> = content.lines()
            .skip(offset.saturating_sub(1))
            .take(limit)
            .enumerate()
            .map(|(i, line)| format!("{:>6}: {}", offset + i, line))
            .collect();
        
        Ok(ToolResult {
            output: lines.join("\n"),
            metadata: json!({ "lines": lines.len(), "path": path.display().to_string() }).as_object().unwrap().clone(),
        })
    }
}
```

### write

```rust
pub struct WriteTool;

impl Tool for WriteTool {
    fn name(&self) -> &str { "write" }
    
    fn description(&self) -> &str {
        "Write content to a new file. Fails if file already exists."
    }
    
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The path where the file should be created"
                },
                "content": {
                    "type": "string",
                    "description": "The content to write to the file"
                }
            },
            "required": ["path", "content"]
        })
    }
    
    fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let path = args["path"].as_str().ok_or(ToolError::InvalidArgs("path required".into()))?;
        let content = args["content"].as_str().ok_or(ToolError::InvalidArgs("content required".into()))?;
        
        let path = ctx.working_dir.join(path);
        
        // Check if file exists
        if path.exists() {
            return Err(ToolError::ExecutionFailed("File already exists. Use edit tool instead.".into()));
        }
        
        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
        }
        
        fs::write(&path, content)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
        
        Ok(ToolResult {
            output: format!("File created: {}", path.display()),
            metadata: HashMap::new(),
        })
    }
}
```

### edit

```rust
pub struct EditTool;

impl Tool for EditTool {
    fn name(&self) -> &str { "edit" }
    
    fn description(&self) -> &str {
        "Edit an existing file by replacing specific text. The old_string must match exactly."
    }
    
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The path to the file to edit"
                },
                "old_string": {
                    "type": "string",
                    "description": "The exact text to find and replace"
                },
                "new_string": {
                    "type": "string",
                    "description": "The text to replace it with"
                },
                "replace_all": {
                    "type": "boolean",
                    "description": "Replace all occurrences",
                    "default": false
                }
            },
            "required": ["path", "old_string", "new_string"]
        })
    }
    
    fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let path = args["path"].as_str().ok_or(ToolError::InvalidArgs("path required".into()))?;
        let old_string = args["old_string"].as_str().ok_or(ToolError::InvalidArgs("old_string required".into()))?;
        let new_string = args["new_string"].as_str().ok_or(ToolError::InvalidArgs("new_string required".into()))?;
        let replace_all = args.get("replace_all").and_then(|v| v.as_bool()).unwrap_or(false);
        
        let path = ctx.working_dir.join(path);
        
        let content = fs::read_to_string(&path)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
        
        let occurrences = content.matches(old_string).count();
        
        if occurrences == 0 {
            return Err(ToolError::ExecutionFailed("old_string not found in file".into()));
        }
        
        if occurrences > 1 && !replace_all {
            return Err(ToolError::ExecutionFailed(format!(
                "Found {} occurrences. Set replace_all=true or provide more context.",
                occurrences
            )));
        }
        
        let new_content = if replace_all {
            content.replace(old_string, new_string)
        } else {
            content.replacen(old_string, new_string, 1)
        };
        
        fs::write(&path, &new_content)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
        
        Ok(ToolResult {
            output: format!("Replaced {} occurrence(s) in {}", occurrences, path.display()),
            metadata: json!({ "occurrences": occurrences }).as_object().unwrap().clone(),
        })
    }
}
```

### bash

```rust
pub struct BashTool;

impl Tool for BashTool {
    fn name(&self) -> &str { "bash" }
    
    fn description(&self) -> &str {
        "Execute a shell command. Use for git, npm, cargo, etc. Not for file operations."
    }
    
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The command to execute"
                },
                "timeout": {
                    "type": "integer",
                    "description": "Timeout in milliseconds",
                    "default": 120000
                }
            },
            "required": ["command"]
        })
    }
    
    fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let command = args["command"].as_str().ok_or(ToolError::InvalidArgs("command required".into()))?;
        let timeout_ms = args.get("timeout").and_then(|v| v.as_u64()).unwrap_or(120000);
        
        let output = Command::new("bash")
            .arg("-c")
            .arg(command)
            .current_dir(&ctx.working_dir)
            .envs(&ctx.env)
            .output()
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        
        let result = if output.status.success() {
            stdout.to_string()
        } else {
            format!("Exit code: {}\n{}", output.status.code().unwrap_or(-1), stderr)
        };
        
        Ok(ToolResult {
            output: result,
            metadata: json!({ "exit_code": output.status.code() }).as_object().unwrap().clone(),
        })
    }
}
```

## Tool Registry

```rust
pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        let mut registry = Self { tools: HashMap::new() };
        registry.register(Box::new(ReadTool));
        registry.register(Box::new(WriteTool));
        registry.register(Box::new(EditTool));
        registry.register(Box::new(BashTool));
        registry
    }
    
    pub fn register(&mut self, tool: Box<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }
    
    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(|t| t.as_ref())
    }
    
    pub fn to_anthropic_tools(&self) -> Vec<Value> {
        self.tools.values().map(|tool| {
            json!({
                "name": tool.name(),
                "description": tool.description(),
                "input_schema": tool.parameters()
            })
        }).collect()
    }
}
```

## Execution Flow

```
LLM Response → Tool Call Request → Validate Args → Execute Tool → Return Result → Next LLM Call
```

## Safety Considerations

1. **Path validation** - Prevent directory traversal attacks
2. **Command allowlist** - Optional bash command restrictions
3. **Timeout enforcement** - Prevent hanging commands
4. **File size limits** - Prevent reading huge files
5. **Working directory** - Restrict to project root

## Future Tools

| Tool | Phase | Description |
|------|-------|-------------|
| `glob` | 2 | File pattern matching |
| `grep` | 2 | Content search |
| `web_search` | 2 | Research capability |
| `simulate` | 3 | Run simulations |
| `verify` | 3 | TLA+ verification |
