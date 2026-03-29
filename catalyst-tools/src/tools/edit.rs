use crate::{Tool, ToolContext, ToolResult};
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::fs;

#[derive(Clone)]
pub struct EditTool;

#[async_trait::async_trait]
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

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult> {
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
