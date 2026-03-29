use crate::{Tool, ToolContext, ToolResult};
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::fs;

#[derive(Clone)]
pub struct WriteTool;

#[async_trait::async_trait]
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

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult> {
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
