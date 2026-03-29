use crate::{Tool, ToolContext, ToolResult};
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};

#[derive(Clone)]
pub struct BashTool;

#[async_trait::async_trait]
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

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult> {
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
