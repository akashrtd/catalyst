use crate::AgentEvent;
use anyhow::{Context, Result};
use catalyst_llm::{Content, ContentBlock, LlmProvider, Message, Role, StreamEvent};
use catalyst_tools::{ToolContext, ToolRegistry};
use serde_json::Value;
use std::collections::HashMap;
use tokio::sync::mpsc;

pub struct Agent {
    provider: Box<dyn LlmProvider + Send + Sync>,
    tools: ToolRegistry,
    messages: Vec<Message>,
    system_prompt: String,
    working_dir: std::path::PathBuf,
}

impl Agent {
    pub fn new(
        provider: Box<dyn LlmProvider + Send + Sync>,
        tools: ToolRegistry,
        working_dir: std::path::PathBuf,
    ) -> Self {
        Self {
            provider,
            tools,
            messages: Vec::new(),
            system_prompt: DEFAULT_SYSTEM_PROMPT.to_string(),
            working_dir,
        }
    }

    pub fn set_system_prompt(&mut self, prompt: String) {
        self.system_prompt = prompt;
    }

    pub async fn send(
        &mut self,
        user_message: String,
        tx: mpsc::UnboundedSender<AgentEvent>,
    ) -> Result<()> {
        self.messages.push(Message {
            role: Role::User,
            content: Content::Text(user_message),
        });

        self.process_stream(tx).await
    }

    async fn process_stream(&mut self, tx: mpsc::UnboundedSender<AgentEvent>) -> Result<()> {
        let anthropic_tools = self.tools.to_anthropic_tools();

        let mut stream = self
            .provider
            .stream(
                Some(&self.system_prompt),
                self.messages.clone(),
                anthropic_tools,
            )
            .await?;

        let mut assistant_content: Vec<ContentBlock> = Vec::new();
        let mut current_text = String::new();
        let mut current_thinking = String::new();
        let mut current_tool_call: Option<(String, String, String)> = None;

        while let Ok(Some(event)) = stream.next_event().await {
            match event {
                StreamEvent::MessageStart { message: _ } => {}

                StreamEvent::ContentBlockStart {
                    index: _,
                    content_block,
                } => match content_block {
                    ContentBlock::Text { text } => {
                        current_text.push_str(&text);
                        let _ = tx.send(AgentEvent::TextDelta { text });
                    }
                    ContentBlock::Thinking { thinking } => {
                        current_thinking.push_str(&thinking);
                        let _ = tx.send(AgentEvent::ThinkingDelta { thinking });
                    }
                    ContentBlock::ToolUse { id, name, input } => {
                        current_tool_call = Some((id, name, String::new()));
                        if !input.is_null() {
                            if let Some((_, _, ref mut args)) = current_tool_call.as_mut() {
                                args.push_str(&input.to_string());
                            }
                        }
                    }
                    ContentBlock::ToolResult { .. } => {}
                    ContentBlock::Redacted { .. } => {}
                },

                StreamEvent::ContentBlockDelta { index: _, delta } => match delta {
                    catalyst_llm::Delta::TextDelta { text } => {
                        current_text.push_str(&text);
                        let _ = tx.send(AgentEvent::TextDelta { text });
                    }
                    catalyst_llm::Delta::ThinkingDelta { thinking } => {
                        current_thinking.push_str(&thinking);
                        let _ = tx.send(AgentEvent::ThinkingDelta { thinking });
                    }
                    catalyst_llm::Delta::InputJsonDelta { partial_json } => {
                        if let Some((_, _, ref mut args)) = current_tool_call.as_mut() {
                            args.push_str(&partial_json);
                        }
                    }
                },

                StreamEvent::ContentBlockStop { index: _ } => {
                    if !current_text.is_empty() {
                        assistant_content.push(ContentBlock::Text {
                            text: current_text.clone(),
                        });
                    }

                    if let Some((id, name, args_json)) = current_tool_call.take() {
                        let args: Value = serde_json::from_str(&args_json)
                            .unwrap_or_else(|_| Value::Object(serde_json::Map::new()));

                        assistant_content.push(ContentBlock::ToolUse {
                            id: id.clone(),
                            name: name.clone(),
                            input: args.clone(),
                        });

                        let _ = tx.send(AgentEvent::ToolCall {
                            id: id.clone(),
                            name: name.clone(),
                            args: args.clone(),
                        });

                        let tools = self.tools.clone();
                        let name_clone = name.clone();
                        let args_clone = args.clone();
                        let working_dir = self.working_dir.clone();

                        let result = tokio::task::spawn_blocking(move || {
                            let ctx = ToolContext {
                                working_dir,
                                env: HashMap::new(),
                                timeout_ms: 120_000,
                            };
                            tools.execute(&name_clone, args_clone, &ctx)
                        })
                        .await
                        .context("Tool execution panicked")??;

                        let (output, is_error) = (result.output, false);

                        let _ = tx.send(AgentEvent::ToolResult {
                            id: id.clone(),
                            result: output.clone(),
                            is_error,
                        });

                        self.messages.push(Message {
                            role: Role::Assistant,
                            content: Content::Blocks(assistant_content.clone()),
                        });

                        self.messages.push(Message {
                            role: Role::User,
                            content: Content::Blocks(vec![ContentBlock::ToolResult {
                                tool_use_id: id,
                                content: output,
                                is_error,
                            }]),
                        });

                        return Box::pin(self.process_stream(tx)).await;
                    }
                }

                StreamEvent::MessageDelta { delta, usage } => {
                    if delta.stop_reason.is_some() {
                        let _ = tx.send(AgentEvent::TokenUsage {
                            input: usage.input_tokens,
                            output: usage.output_tokens,
                        });
                    }
                }

                StreamEvent::MessageStop => {
                    if !assistant_content.is_empty() {
                        self.messages.push(Message {
                            role: Role::Assistant,
                            content: Content::Blocks(assistant_content.clone()),
                        });
                    }
                    let _ = tx.send(AgentEvent::Complete);
                }

                StreamEvent::Error { error } => {
                    let _ = tx.send(AgentEvent::Error(error.message));
                }
            }
        }

        Ok(())
    }
}

const DEFAULT_SYSTEM_PROMPT: &str = r#"
You are Catalyst, a research-driven AI coding agent.

Your philosophy:
- Research best practices before making changes
- Explain WHY you make each choice
- Challenge user assumptions when wrong
- Prioritize correctness over speed
- Write stable, secure, flawless code

When editing code:
1. Read the relevant files first
2. Understand the context
3. Make minimal, focused changes
4. Verify your changes work

Available tools:
- read: Read file contents
- write: Create new files
- edit: Edit existing files
- bash: Execute shell commands

Always think through problems carefully before acting.
"#;
