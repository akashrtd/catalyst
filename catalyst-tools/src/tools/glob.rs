use crate::{Tool, ToolContext, ToolResult};
use anyhow::{Context, Result};
use serde_json::{json, Value};

#[derive(Clone)]
pub struct GlobTool;

#[async_trait::async_trait]
impl Tool for GlobTool {
    fn name(&self) -> &str {
        "glob"
    }

    fn description(&self) -> &str {
        "Find files matching a glob pattern. Returns matched file paths relative to working directory, sorted by modification time (most recent first)."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Glob pattern (e.g. **/*.rs, src/**/*.ts, **/test*)"
                },
                "max_results": {
                    "type": "integer",
                    "description": "Maximum number of results to return",
                    "default": 100
                }
            },
            "required": ["pattern"]
        })
    }

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult> {
        let pattern = args["pattern"].as_str().context("pattern required")?;
        let max_results = args
            .get("max_results")
            .and_then(|v| v.as_u64())
            .unwrap_or(100) as usize;

        let working_dir = ctx.working_dir.clone();
        let full_pattern = working_dir.join(pattern);
        let pattern_str = full_pattern
            .to_str()
            .context("Invalid glob pattern (non-UTF-8 path)")?
            .to_string();

        let entries: Vec<_> = glob::glob(&pattern_str)
            .with_context(|| format!("Invalid glob pattern: {}", pattern))?
            .filter_map(|entry| entry.ok())
            .filter_map(|path| {
                let relative = path.strip_prefix(&working_dir).ok()?;
                let metadata = path.metadata().ok()?;
                let modified = metadata.modified().ok()?;
                Some((relative.to_path_buf(), modified))
            })
            .collect();

        let mut entries = entries;
        entries.sort_by(|a, b| b.1.cmp(&a.1));

        let results: Vec<String> = entries
            .into_iter()
            .take(max_results)
            .map(|(path, _)| path.display().to_string())
            .collect();

        if results.is_empty() {
            Ok(ToolResult::success("No files matched the pattern."))
        } else {
            let output = format!("Found {} file(s):\n{}", results.len(), results.join("\n"));
            Ok(ToolResult::success(output))
        }
    }

    fn clone_box(&self) -> Box<dyn Tool> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashMap;
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

    #[tokio::test]
    async fn test_glob_finds_rs_files() {
        let (ctx, _temp) = create_test_context();
        fs::write(ctx.working_dir.join("foo.rs"), "fn main() {}").unwrap();
        fs::write(ctx.working_dir.join("bar.rs"), "fn bar() {}").unwrap();
        fs::write(ctx.working_dir.join("baz.txt"), "hello").unwrap();

        let tool = GlobTool;
        let result = tool
            .execute(json!({"pattern": "*.rs"}), &ctx)
            .await
            .unwrap();

        assert!(result.output.contains("foo.rs"));
        assert!(result.output.contains("bar.rs"));
        assert!(!result.output.contains("baz.txt"));
    }

    #[tokio::test]
    async fn test_glob_recursive_pattern() {
        let (ctx, _temp) = create_test_context();
        fs::create_dir_all(ctx.working_dir.join("src")).unwrap();
        fs::write(ctx.working_dir.join("src/main.rs"), "fn main() {}").unwrap();
        fs::write(ctx.working_dir.join("README.md"), "# test").unwrap();

        let tool = GlobTool;
        let result = tool
            .execute(json!({"pattern": "**/*.rs"}), &ctx)
            .await
            .unwrap();

        assert!(result.output.contains("src/main.rs"));
        assert!(!result.output.contains("README.md"));
    }

    #[tokio::test]
    async fn test_glob_no_matches() {
        let (ctx, _temp) = create_test_context();
        fs::write(ctx.working_dir.join("test.txt"), "hello").unwrap();

        let tool = GlobTool;
        let result = tool
            .execute(json!({"pattern": "*.xyz"}), &ctx)
            .await
            .unwrap();

        assert!(result.output.contains("No files matched"));
    }

    #[tokio::test]
    async fn test_glob_max_results() {
        let (ctx, _temp) = create_test_context();
        for i in 0..10 {
            fs::write(
                ctx.working_dir.join(format!("file_{:02}.txt", i)),
                "content",
            )
            .unwrap();
        }

        let tool = GlobTool;
        let result = tool
            .execute(json!({"pattern": "*.txt", "max_results": 3}), &ctx)
            .await
            .unwrap();

        let lines: Vec<&str> = result.output.lines().collect();
        assert_eq!(lines.len(), 4);
    }
}
