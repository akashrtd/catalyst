use crate::{Tool, ToolContext, ToolResult};
use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::fs;

#[derive(Clone)]
pub struct ListTool;

const SKIP_DIRS: &[&str] = &[
    "target",
    "node_modules",
    ".git",
    ".next",
    "dist",
    "build",
    "__pycache__",
    ".cache",
];

#[async_trait::async_trait]
impl Tool for ListTool {
    fn name(&self) -> &str {
        "list"
    }

    fn description(&self) -> &str {
        "List directory contents with file metadata. Returns file names, sizes, and types. Automatically skips common build artifact directories."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Directory to list (default: working directory)"
                },
                "recursive": {
                    "type": "boolean",
                    "description": "List recursively",
                    "default": false
                },
                "max_depth": {
                    "type": "integer",
                    "description": "Maximum recursion depth",
                    "default": 3
                }
            },
            "required": []
        })
    }

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult> {
        let list_path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
        let recursive = args
            .get("recursive")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let max_depth = args.get("max_depth").and_then(|v| v.as_u64()).unwrap_or(3) as usize;

        let target = ctx.working_dir.join(list_path);
        let canonical = target
            .canonicalize()
            .with_context(|| format!("Path does not exist: {}", target.display()))?;
        let canonical_working = ctx
            .working_dir
            .canonicalize()
            .context("Working directory does not exist")?;

        if !canonical.starts_with(&canonical_working) {
            anyhow::bail!(
                "Path '{}' is outside working directory",
                canonical.display()
            );
        }

        if !canonical.is_dir() {
            anyhow::bail!("Path is not a directory: {}", canonical.display());
        }

        let mut entries: Vec<String> = Vec::new();
        list_dir(&canonical, "", recursive, max_depth, 0, &mut entries);

        if entries.is_empty() {
            Ok(ToolResult::success("Empty directory."))
        } else {
            let output = format!("{} entries:\n{}", entries.len(), entries.join("\n"));
            Ok(ToolResult::success(output))
        }
    }

    fn clone_box(&self) -> Box<dyn Tool> {
        Box::new(self.clone())
    }
}

fn list_dir(
    dir: &std::path::Path,
    prefix: &str,
    recursive: bool,
    max_depth: usize,
    current_depth: usize,
    entries: &mut Vec<String>,
) {
    let read_dir = match fs::read_dir(dir) {
        Ok(r) => r,
        Err(_) => return,
    };

    let mut dir_entries: Vec<_> = read_dir
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name();
            let name_str = name.to_string_lossy();
            if current_depth == 0 && SKIP_DIRS.contains(&name_str.as_ref()) {
                return false;
            }
            // Also skip hidden dirs at any depth
            if name_str.starts_with('.') && e.path().is_dir() {
                return false;
            }
            true
        })
        .collect();

    // Sort: directories first, then files, alphabetically within each group
    dir_entries.sort_by(|a, b| {
        let a_is_dir = a.file_type().map(|t| t.is_dir()).unwrap_or(false);
        let b_is_dir = b.file_type().map(|t| t.is_dir()).unwrap_or(false);
        match (a_is_dir, b_is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.file_name().cmp(&b.file_name()),
        }
    });

    for entry in dir_entries {
        let name = entry.file_name().to_string_lossy().to_string();
        let path = entry.path();
        let display_prefix = if prefix.is_empty() {
            String::new()
        } else {
            format!("{}/", prefix)
        };

        let metadata = entry.metadata().ok();
        let is_dir = metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false);

        if is_dir {
            entries.push(format!("{}{}/", display_prefix, name));
            if recursive && current_depth < max_depth {
                list_dir(
                    &path,
                    &format!("{}{}", display_prefix, name),
                    recursive,
                    max_depth,
                    current_depth + 1,
                    entries,
                );
            }
        } else {
            let size_str = metadata
                .as_ref()
                .map(|m| format_size(m.len()))
                .unwrap_or_else(|| "?".to_string());
            entries.push(format!("{}{} ({})", display_prefix, name, size_str));
        }
    }
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;

    if bytes < KB {
        format!("{}B", bytes)
    } else if bytes < MB {
        format!("{:.1}KB", bytes as f64 / KB as f64)
    } else {
        format!("{:.1}MB", bytes as f64 / MB as f64)
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
    async fn test_list_basic() {
        let (ctx, _temp) = create_test_context();
        fs::write(ctx.working_dir.join("file.txt"), "hello").unwrap();
        fs::create_dir_all(ctx.working_dir.join("src")).unwrap();
        fs::write(ctx.working_dir.join("src/main.rs"), "fn main() {}").unwrap();

        let tool = ListTool;
        let result = tool.execute(json!({}), &ctx).await.unwrap();

        assert!(result.output.contains("src/"));
        assert!(result.output.contains("file.txt"));
    }

    #[tokio::test]
    async fn test_list_recursive() {
        let (ctx, _temp) = create_test_context();
        fs::create_dir_all(ctx.working_dir.join("src/utils")).unwrap();
        fs::write(ctx.working_dir.join("src/main.rs"), "fn main() {}").unwrap();
        fs::write(ctx.working_dir.join("src/utils/helpers.rs"), "fn help() {}").unwrap();

        let tool = ListTool;
        let result = tool
            .execute(json!({"recursive": true}), &ctx)
            .await
            .unwrap();

        assert!(result.output.contains("src/"));
        assert!(result.output.contains("src/main.rs"));
        assert!(result.output.contains("src/utils/helpers.rs"));
    }

    #[tokio::test]
    async fn test_list_skips_hidden_and_build_dirs() {
        let (ctx, _temp) = create_test_context();
        fs::create_dir_all(ctx.working_dir.join("target")).unwrap();
        fs::write(ctx.working_dir.join("target/build.log"), "log").unwrap();
        fs::create_dir_all(ctx.working_dir.join(".hidden")).unwrap();
        fs::write(ctx.working_dir.join("README.md"), "# test").unwrap();

        let tool = ListTool;
        let result = tool.execute(json!({}), &ctx).await.unwrap();

        assert!(result.output.contains("README.md"));
        assert!(!result.output.contains("target"));
        assert!(!result.output.contains(".hidden"));
    }

    #[tokio::test]
    async fn test_list_max_depth() {
        let (ctx, _temp) = create_test_context();
        fs::create_dir_all(ctx.working_dir.join("a/b/c/d")).unwrap();
        fs::write(ctx.working_dir.join("a/b/c/d/deep.txt"), "deep").unwrap();
        fs::write(ctx.working_dir.join("a/shallow.txt"), "shallow").unwrap();

        let tool = ListTool;
        let result = tool
            .execute(json!({"recursive": true, "max_depth": 2}), &ctx)
            .await
            .unwrap();

        assert!(result.output.contains("shallow.txt"));
        assert!(!result.output.contains("deep.txt"));
    }

    #[tokio::test]
    async fn test_list_specific_path() {
        let (ctx, _temp) = create_test_context();
        fs::create_dir_all(ctx.working_dir.join("src")).unwrap();
        fs::write(ctx.working_dir.join("src/a.rs"), "a").unwrap();
        fs::write(ctx.working_dir.join("src/b.rs"), "b").unwrap();
        fs::write(ctx.working_dir.join("root.txt"), "root").unwrap();

        let tool = ListTool;
        let result = tool.execute(json!({"path": "src"}), &ctx).await.unwrap();

        assert!(result.output.contains("a.rs"));
        assert!(result.output.contains("b.rs"));
        assert!(!result.output.contains("root.txt"));
    }

    #[tokio::test]
    async fn test_list_file_size_format() {
        let (ctx, _temp) = create_test_context();
        fs::write(ctx.working_dir.join("small.txt"), "hi").unwrap();

        let tool = ListTool;
        let result = tool.execute(json!({}), &ctx).await.unwrap();

        assert!(result.output.contains("B)"));
    }

    #[tokio::test]
    async fn test_list_empty_directory() {
        let (ctx, _temp) = create_test_context();
        let tool = ListTool;
        let result = tool.execute(json!({}), &ctx).await.unwrap();

        assert!(result.output.contains("Empty directory"));
    }

    #[tokio::test]
    async fn test_list_nonexistent_path() {
        let (ctx, _temp) = create_test_context();

        let tool = ListTool;
        let result = tool.execute(json!({"path": "nonexistent"}), &ctx).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_format_size() {
        assert_eq!(format_size(0), "0B");
        assert_eq!(format_size(512), "512B");
        assert_eq!(format_size(1024), "1.0KB");
        assert_eq!(format_size(1048576), "1.0MB");
    }
}
