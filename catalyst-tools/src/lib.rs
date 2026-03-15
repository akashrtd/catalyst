pub mod registry;
pub mod tools;

pub use registry::*;
pub use tools::*;

use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Clone)]
pub struct ToolContext {
    pub working_dir: std::path::PathBuf,
    pub env: HashMap<String, String>,
    pub timeout_ms: u64,
}

#[derive(Clone)]
pub struct ToolResult {
    pub output: String,
    pub metadata: HashMap<String, Value>,
}

impl ToolResult {
    pub fn success(output: impl Into<String>) -> Self {
        Self {
            output: output.into(),
            metadata: HashMap::new(),
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            output: format!("Error: {}", message.into()),
            metadata: HashMap::new(),
        }
    }
}

pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> Value;
    fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult>;

    fn clone_box(&self) -> Box<dyn Tool>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_context() -> (ToolContext, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let ctx = ToolContext {
            working_dir: temp_dir.path().to_path_buf(),
            env: HashMap::new(),
            timeout_ms: 5000,
        };
        (ctx, temp_dir)
    }

    #[test]
    fn test_read_tool() {
        let (ctx, _temp) = create_test_context();
        let test_file = ctx.working_dir.join("test.txt");
        fs::write(&test_file, "line1\nline2\nline3").unwrap();

        let tool = ReadTool;
        let result = tool.execute(json!({"path": "test.txt"}), &ctx).unwrap();

        assert!(result.output.contains("line1"));
        assert!(result.output.contains("line2"));
        assert!(result.output.contains("line3"));
    }

    #[test]
    fn test_read_tool_with_offset() {
        let (ctx, _temp) = create_test_context();
        let test_file = ctx.working_dir.join("test.txt");
        fs::write(&test_file, "line1\nline2\nline3").unwrap();

        let tool = ReadTool;
        let result = tool
            .execute(json!({"path": "test.txt", "offset": 2}), &ctx)
            .unwrap();

        assert!(!result.output.contains("line1"));
        assert!(result.output.contains("line2"));
        assert!(result.output.contains("line3"));
    }

    #[test]
    fn test_write_tool() {
        let (ctx, _temp) = create_test_context();
        let tool = WriteTool;

        let result = tool
            .execute(json!({"path": "new.txt", "content": "hello world"}), &ctx)
            .unwrap();

        assert!(result.output.contains("File created"));
        assert!(ctx.working_dir.join("new.txt").exists());

        let content = fs::read_to_string(ctx.working_dir.join("new.txt")).unwrap();
        assert_eq!(content, "hello world");
    }

    #[test]
    fn test_write_tool_fails_on_existing() {
        let (ctx, _temp) = create_test_context();
        let test_file = ctx.working_dir.join("existing.txt");
        fs::write(&test_file, "old content").unwrap();

        let tool = WriteTool;
        let result = tool.execute(
            json!({"path": "existing.txt", "content": "new content"}),
            &ctx,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_edit_tool() {
        let (ctx, _temp) = create_test_context();
        let test_file = ctx.working_dir.join("test.txt");
        fs::write(&test_file, "hello world").unwrap();

        let tool = EditTool;
        let result = tool
            .execute(
                json!({"path": "test.txt", "old_string": "world", "new_string": "rust"}),
                &ctx,
            )
            .unwrap();

        assert!(result.output.contains("Replaced 1 occurrence"));

        let content = fs::read_to_string(test_file).unwrap();
        assert_eq!(content, "hello rust");
    }

    #[test]
    fn test_edit_tool_multiple_occurrences() {
        let (ctx, _temp) = create_test_context();
        let test_file = ctx.working_dir.join("test.txt");
        fs::write(&test_file, "foo foo foo").unwrap();

        let tool = EditTool;
        let result = tool.execute(
            json!({"path": "test.txt", "old_string": "foo", "new_string": "bar"}),
            &ctx,
        );

        assert!(result.is_err());

        let _result = tool.execute(
            json!({"path": "test.txt", "old_string": "foo", "new_string": "bar", "replace_all": true}),
            &ctx,
        ).unwrap();

        let content = fs::read_to_string(test_file).unwrap();
        assert_eq!(content, "bar bar bar");
    }

    #[test]
    fn test_bash_tool_simple_command() {
        let (ctx, _temp) = create_test_context();
        let tool = BashTool;

        let result = tool
            .execute(json!({"command": "echo hello"}), &ctx)
            .unwrap();
        assert!(result.output.contains("hello"));
    }

    #[test]
    fn test_bash_tool_blocks_dangerous_commands() {
        let (ctx, _temp) = create_test_context();
        let tool = BashTool;

        let result = tool.execute(json!({"command": "rm -rf /"}), &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_tool_registry() {
        let registry = ToolRegistry::new();

        assert!(registry.get("read").is_some());
        assert!(registry.get("write").is_some());
        assert!(registry.get("edit").is_some());
        assert!(registry.get("bash").is_some());
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn test_tool_registry_anthropic_tools() {
        let registry = ToolRegistry::new();
        let tools = registry.to_anthropic_tools();

        assert_eq!(tools.len(), 4);

        let names: Vec<&str> = tools
            .iter()
            .filter_map(|t| t.get("name").and_then(|n| n.as_str()))
            .collect();

        assert!(names.contains(&"read"));
        assert!(names.contains(&"write"));
        assert!(names.contains(&"edit"));
        assert!(names.contains(&"bash"));
    }
}
