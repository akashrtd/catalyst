use crate::tools::{BashTool, EditTool, GlobTool, GrepTool, ListTool, ReadTool, WriteTool};
use crate::{Tool, ToolContext, ToolResult};
use anyhow::{Context, Result};
use serde_json::json;
use std::collections::HashMap;

pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            tools: HashMap::new(),
        };

        registry.register(Box::new(ReadTool));
        registry.register(Box::new(WriteTool));
        registry.register(Box::new(EditTool));
        registry.register(Box::new(BashTool));
        registry.register(Box::new(GlobTool));
        registry.register(Box::new(GrepTool));
        registry.register(Box::new(ListTool));

        registry
    }

    pub fn register(&mut self, tool: Box<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(|t| t.as_ref())
    }

    pub fn to_anthropic_tools(&self) -> Vec<serde_json::Value> {
        self.tools
            .values()
            .map(|tool| {
                json!({
                    "name": tool.name(),
                    "description": tool.description(),
                    "input_schema": tool.parameters()
                })
            })
            .collect()
    }

    pub async fn execute(
        &self,
        name: &str,
        args: serde_json::Value,
        ctx: &ToolContext,
    ) -> Result<ToolResult> {
        let tool = self
            .get(name)
            .with_context(|| format!("Unknown tool: {}", name))?;

        let mut result = tool.execute(args, ctx).await?;

        let limit = tool.output_limit();
        if result.output.len() > limit {
            result.output = result.truncate_for_context(limit);
        }

        Ok(result)
    }
}

impl Clone for ToolRegistry {
    fn clone(&self) -> Self {
        Self {
            tools: self
                .tools
                .iter()
                .map(|(k, v)| (k.clone(), v.clone_box()))
                .collect(),
        }
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}
