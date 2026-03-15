use crate::{Tool, ToolContext, ToolResult};
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::fs;

#[derive(Clone)]
pub struct ReadTool;

impl Tool for ReadTool {
    fn name(&self) -> &str {
        "read"
    }

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

    fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult> {
        let path = args["path"].as_str().context("path required")?;

        let path = ctx.working_dir.join(path);

        let canonical_path = path
            .canonicalize()
            .with_context(|| format!("Path does not exist: {}", path.display()))?;
        let canonical_working_dir = ctx
            .working_dir
            .canonicalize()
            .context("Working directory does not exist")?;

        if !canonical_path.starts_with(&canonical_working_dir) {
            bail!(
                "Path '{}' is outside working directory",
                canonical_path.display()
            );
        }

        let offset = args.get("offset").and_then(|v| v.as_u64()).unwrap_or(1) as usize;

        let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(2000) as usize;

        let content = fs::read_to_string(&canonical_path)
            .with_context(|| format!("Failed to read file: {}", canonical_path.display()))?;

        let lines: Vec<_> = content
            .lines()
            .skip(offset.saturating_sub(1))
            .take(limit)
            .enumerate()
            .map(|(i, line)| format!("{:>6}\t{}", offset + i, line))
            .collect();

        Ok(ToolResult::success(lines.join("\n")))
    }

    fn clone_box(&self) -> Box<dyn Tool> {
        Box::new(self.clone())
    }
}

#[derive(Clone)]
pub struct WriteTool;

impl Tool for WriteTool {
    fn name(&self) -> &str {
        "write"
    }

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

    fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult> {
        let path = args["path"].as_str().context("path required")?;
        let content = args["content"].as_str().context("content required")?;

        let path = ctx.working_dir.join(path);

        if path.exists() {
            bail!(
                "File already exists: {}. Use edit tool instead.",
                path.display()
            );
        }

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).context("Failed to create parent directories")?;
        }

        fs::write(&path, content)
            .with_context(|| format!("Failed to write file: {}", path.display()))?;

        Ok(ToolResult::success(format!(
            "File created: {}",
            path.display()
        )))
    }

    fn clone_box(&self) -> Box<dyn Tool> {
        Box::new(self.clone())
    }
}

#[derive(Clone)]
pub struct EditTool;

impl Tool for EditTool {
    fn name(&self) -> &str {
        "edit"
    }

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

    fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult> {
        let path = args["path"].as_str().context("path required")?;
        let old_string = args["old_string"].as_str().context("old_string required")?;
        let new_string = args["new_string"].as_str().context("new_string required")?;
        let replace_all = args
            .get("replace_all")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let path = ctx.working_dir.join(path);

        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read file: {}", path.display()))?;

        let occurrences = content.matches(old_string).count();

        if occurrences == 0 {
            bail!("old_string not found in file: {}", path.display());
        }

        if occurrences > 1 && !replace_all {
            bail!(
                "Found {} occurrences. Set replace_all=true or provide more context.",
                occurrences
            );
        }

        let new_content = if replace_all {
            content.replace(old_string, new_string)
        } else {
            content.replacen(old_string, new_string, 1)
        };

        fs::write(&path, &new_content)
            .with_context(|| format!("Failed to write file: {}", path.display()))?;

        Ok(ToolResult::success(format!(
            "Replaced {} occurrence(s) in {}",
            occurrences,
            path.display()
        )))
    }

    fn clone_box(&self) -> Box<dyn Tool> {
        Box::new(self.clone())
    }
}

#[derive(Clone)]
pub struct BashTool;

impl Tool for BashTool {
    fn name(&self) -> &str {
        "bash"
    }

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

    fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult> {
        let command = args["command"].as_str().context("command required")?;

        let dangerous_patterns = ["rm -rf /", "sudo rm", "> /dev/sd"];
        for pattern in &dangerous_patterns {
            if command.contains(pattern) {
                bail!("Blocked dangerous command pattern: {}", pattern);
            }
        }

        let working_dir = ctx.working_dir.clone();
        let command = command.to_string();

        let output = std::thread::spawn(move || {
            std::process::Command::new("bash")
                .arg("-c")
                .arg(&command)
                .current_dir(&working_dir)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .output()
        })
        .join()
        .map_err(|_| anyhow::anyhow!("Command execution panicked"))?
        .context("Failed to execute command")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        let result = if output.status.success() {
            stdout.to_string()
        } else {
            format!(
                "Exit code: {}\n{}\n{}",
                output.status.code().unwrap_or(-1),
                stderr,
                stdout
            )
        };

        Ok(ToolResult::success(result))
    }

    fn clone_box(&self) -> Box<dyn Tool> {
        Box::new(self.clone())
    }
}
