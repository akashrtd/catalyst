use crate::{Tool, ToolContext, ToolResult};
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::fs;

#[derive(Clone)]
pub struct ReadTool;

#[async_trait::async_trait]
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

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult> {
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
