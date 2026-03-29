use crate::{Tool, ToolContext, ToolResult};
use anyhow::{Context, Result};
use regex::Regex;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

#[derive(Clone)]
pub struct GrepTool;

#[async_trait::async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &str {
        "grep"
    }

    fn description(&self) -> &str {
        "Search file contents using regex. Returns matching lines with file paths and line numbers."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Regex pattern to search for"
                },
                "path": {
                    "type": "string",
                    "description": "Directory or file to search in (default: working directory)"
                },
                "include": {
                    "type": "string",
                    "description": "File glob to include (e.g. *.rs, *.ts)"
                },
                "max_results": {
                    "type": "integer",
                    "description": "Maximum matching lines to return",
                    "default": 50
                }
            },
            "required": ["pattern"]
        })
    }

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult> {
        let pattern = args["pattern"].as_str().context("pattern required")?;
        let search_path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
        let include_glob = args.get("include").and_then(|v| v.as_str());
        let max_results = args
            .get("max_results")
            .and_then(|v| v.as_u64())
            .unwrap_or(50) as usize;

        let re =
            Regex::new(pattern).with_context(|| format!("Invalid regex pattern: {}", pattern))?;

        let search_root = ctx.working_dir.join(search_path);
        if !search_root.exists() {
            anyhow::bail!("Path does not exist: {}", search_root.display());
        }

        let mut results: Vec<String> = Vec::new();

        if search_root.is_file() {
            search_file(
                &search_root,
                &ctx.working_dir,
                &re,
                &mut results,
                max_results,
            );
        } else {
            search_dir(
                &search_root,
                &ctx.working_dir,
                &re,
                include_glob,
                &mut results,
                max_results,
            );
        }

        if results.is_empty() {
            Ok(ToolResult::success("No matches found."))
        } else {
            let output = format!("Found {} match(es):\n{}", results.len(), results.join("\n"));
            Ok(ToolResult::success(output))
        }
    }

    fn clone_box(&self) -> Box<dyn Tool> {
        Box::new(self.clone())
    }
}

fn search_file(
    file_path: &Path,
    working_dir: &Path,
    re: &Regex,
    results: &mut Vec<String>,
    max_results: usize,
) {
    if results.len() >= max_results {
        return;
    }

    let Ok(content) = fs::read_to_string(file_path) else {
        return;
    };
    let relative = file_path.strip_prefix(working_dir).unwrap_or(file_path);

    for (i, line) in content.lines().enumerate() {
        if results.len() >= max_results {
            break;
        }
        if re.is_match(line) {
            results.push(format!("{}:{}: {}", relative.display(), i + 1, line));
        }
    }
}

fn search_dir(
    dir: &Path,
    working_dir: &Path,
    re: &Regex,
    include_glob: Option<&str>,
    results: &mut Vec<String>,
    max_results: usize,
) {
    let skip_dirs = ["target", "node_modules", ".git", ".next", "dist", "build"];

    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    let include_matcher = include_glob.map(glob::Pattern::new).transpose();

    for entry in entries.flatten() {
        if results.len() >= max_results {
            break;
        }

        let path = entry.path();

        if path.is_dir() {
            let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if skip_dirs.contains(&dir_name) {
                continue;
            }
            search_dir(&path, working_dir, re, include_glob, results, max_results);
        } else if path.is_file() {
            if let Ok(Some(ref matcher)) = include_matcher {
                let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if !matcher.matches(file_name) {
                    continue;
                }
            }
            search_file(&path, working_dir, re, results, max_results);
        }
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
    async fn test_grep_finds_matches() {
        let (ctx, _temp) = create_test_context();
        fs::write(
            ctx.working_dir.join("test.rs"),
            "fn main() {}\nfn foo() {}\n",
        )
        .unwrap();

        let tool = GrepTool;
        let result = tool.execute(json!({"pattern": "fn"}), &ctx).await.unwrap();

        assert!(result.output.contains("test.rs:1:"));
        assert!(result.output.contains("test.rs:2:"));
    }

    #[tokio::test]
    async fn test_grep_regex_pattern() {
        let (ctx, _temp) = create_test_context();
        fs::write(
            ctx.working_dir.join("test.txt"),
            "hello world\nfoo bar\nbaz qux\n",
        )
        .unwrap();

        let tool = GrepTool;
        let result = tool
            .execute(json!({"pattern": "foo|baz"}), &ctx)
            .await
            .unwrap();

        assert!(result.output.contains("foo bar"));
        assert!(result.output.contains("baz qux"));
        assert!(!result.output.contains("hello"));
    }

    #[tokio::test]
    async fn test_grep_include_filter() {
        let (ctx, _temp) = create_test_context();
        fs::write(ctx.working_dir.join("code.rs"), "fn test() {}").unwrap();
        fs::write(ctx.working_dir.join("notes.txt"), "test notes").unwrap();

        let tool = GrepTool;
        let result = tool
            .execute(json!({"pattern": "test", "include": "*.rs"}), &ctx)
            .await
            .unwrap();

        assert!(result.output.contains("code.rs"));
        assert!(!result.output.contains("notes.txt"));
    }

    #[tokio::test]
    async fn test_grep_no_matches() {
        let (ctx, _temp) = create_test_context();
        fs::write(ctx.working_dir.join("test.txt"), "hello world").unwrap();

        let tool = GrepTool;
        let result = tool
            .execute(json!({"pattern": "xyz123"}), &ctx)
            .await
            .unwrap();

        assert!(result.output.contains("No matches found"));
    }

    #[tokio::test]
    async fn test_grep_specific_file() {
        let (ctx, _temp) = create_test_context();
        fs::write(ctx.working_dir.join("a.txt"), "match here").unwrap();
        fs::write(ctx.working_dir.join("b.txt"), "no match").unwrap();

        let tool = GrepTool;
        let result = tool
            .execute(json!({"pattern": "match", "path": "a.txt"}), &ctx)
            .await
            .unwrap();

        assert!(result.output.contains("a.txt:1:"));
        assert!(!result.output.contains("b.txt"));
    }

    #[tokio::test]
    async fn test_grep_max_results() {
        let (ctx, _temp) = create_test_context();
        let content: Vec<String> = (0..10).map(|i| format!("match_{}", i)).collect();
        fs::write(ctx.working_dir.join("test.txt"), content.join("\n")).unwrap();

        let tool = GrepTool;
        let result = tool
            .execute(json!({"pattern": "match", "max_results": 3}), &ctx)
            .await
            .unwrap();

        assert!(result.output.contains("Found 3 match(es)"));
    }

    #[tokio::test]
    async fn test_grep_skips_target_dir() {
        let (ctx, _temp) = create_test_context();
        fs::create_dir_all(ctx.working_dir.join("target")).unwrap();
        fs::create_dir_all(ctx.working_dir.join("src")).unwrap();
        fs::write(ctx.working_dir.join("target/build.rs"), "fn find_me() {}").unwrap();
        fs::write(ctx.working_dir.join("src/main.rs"), "fn find_me() {}").unwrap();

        let tool = GrepTool;
        let result = tool
            .execute(json!({"pattern": "find_me"}), &ctx)
            .await
            .unwrap();

        assert!(result.output.contains("src/main.rs"));
        assert!(!result.output.contains("target/build.rs"));
    }

    #[tokio::test]
    async fn test_grep_invalid_regex() {
        let (ctx, _temp) = create_test_context();

        let tool = GrepTool;
        let result = tool.execute(json!({"pattern": "[invalid"}), &ctx).await;

        assert!(result.is_err());
    }
}
