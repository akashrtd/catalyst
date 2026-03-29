use crate::{AgentEvent, AgentState};
use anyhow::Result;
use catalyst_llm::{Content, ContentBlock, LlmProvider, Message, Role, StreamEvent};
use catalyst_tools::{ToolContext, ToolRegistry};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;

pub struct AgentConfig {
    pub max_iterations: usize,
    pub auto_retry: bool,
    pub max_retries: usize,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_iterations: 25,
            auto_retry: true,
            max_retries: 2,
        }
    }
}

pub struct Agent {
    provider: Box<dyn LlmProvider + Send + Sync>,
    tools: ToolRegistry,
    messages: Vec<Message>,
    system_prompt: String,
    working_dir: std::path::PathBuf,
    config: AgentConfig,
    state: AgentState,
    cancelled: Arc<AtomicBool>,
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
            config: AgentConfig::default(),
            state: AgentState::Idle,
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn state(&self) -> &AgentState {
        &self.state
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    fn check_cancelled(&self, tx: &mpsc::UnboundedSender<AgentEvent>) -> bool {
        if self.is_cancelled() {
            let _ = tx.send(AgentEvent::Cancelled);
            return true;
        }
        false
    }

    fn transition(&mut self, new_state: AgentState, tx: &mpsc::UnboundedSender<AgentEvent>) {
        let from = self.state.to_string();
        let to = new_state.to_string();
        self.state = new_state;
        let _ = tx.send(AgentEvent::StateChanged { from, to });
    }

    pub fn set_system_prompt(&mut self, prompt: String) {
        self.system_prompt = prompt;
    }

    pub async fn send(
        &mut self,
        user_message: String,
        tx: mpsc::UnboundedSender<AgentEvent>,
    ) -> Result<()> {
        self.cancelled.store(false, Ordering::SeqCst);
        self.transition(AgentState::Planning, &tx);

        self.messages.push(Message {
            role: Role::User,
            content: Content::Text(user_message),
        });

        self.process_stream(tx, 0).await
    }

    async fn process_stream(
        &mut self,
        tx: mpsc::UnboundedSender<AgentEvent>,
        iteration: usize,
    ) -> Result<()> {
        if self.check_cancelled(&tx) {
            self.state = AgentState::Cancelled;
            return Ok(());
        }

        self.transition(AgentState::Executing { iteration }, &tx);

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

                        let ctx = ToolContext {
                            working_dir,
                            env: HashMap::new(),
                            timeout_ms: 120_000,
                        };
                        let result = match tools.execute(&name_clone, args_clone, &ctx).await {
                            Ok(r) => (r.output, false),
                            Err(e) => {
                                let error_msg = format!("Tool error: {}", e);
                                if self.config.auto_retry {
                                    (error_msg, true)
                                } else {
                                    return Err(e);
                                }
                            }
                        };
                        let (output, is_error) = result;

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

                        if iteration >= self.config.max_iterations {
                            let _ = tx.send(AgentEvent::Error(format!(
                                "Max iterations ({}) reached. Stopping to prevent runaway.",
                                self.config.max_iterations
                            )));
                            return Ok(());
                        }

                        if self.check_cancelled(&tx) {
                            self.state = AgentState::Cancelled;
                            return Ok(());
                        }

                        return Box::pin(self.process_stream(tx, iteration + 1)).await;
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
                    self.transition(AgentState::Complete, &tx);
                    let _ = tx.send(AgentEvent::Complete);
                }

                StreamEvent::Error { error } => {
                    self.transition(AgentState::Error(error.message.clone()), &tx);
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
- read: Read file contents with line numbers
- write: Create new files
- edit: Edit existing files by replacing text
- bash: Execute shell commands
- glob: Find files matching a pattern
- grep: Search file contents with regex
- list: List directory contents with metadata

Always think through problems carefully before acting.
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use catalyst_llm::{ContentBlock, LlmProvider, LlmStream, Usage};
    use std::sync::{Arc, Mutex};
    use tokio::sync::mpsc;

    struct MockStream {
        events: Arc<Mutex<Vec<StreamEvent>>>,
    }

    impl MockStream {
        fn new(events: Vec<StreamEvent>) -> Self {
            Self {
                events: Arc::new(Mutex::new(events)),
            }
        }
    }

    #[async_trait::async_trait]
    impl LlmStream for MockStream {
        async fn next_event(&mut self) -> Result<Option<StreamEvent>> {
            let mut events = self.events.lock().unwrap();
            if events.is_empty() {
                Ok(None)
            } else {
                Ok(Some(events.remove(0)))
            }
        }
    }

    struct MockProvider {
        events: Arc<Mutex<Vec<StreamEvent>>>,
        model: String,
    }

    impl MockProvider {
        fn new(model: String, events: Vec<StreamEvent>) -> Self {
            Self {
                events: Arc::new(Mutex::new(events)),
                model,
            }
        }
    }

    #[async_trait::async_trait]
    impl LlmProvider for MockProvider {
        fn name(&self) -> &str {
            "mock"
        }

        fn model(&self) -> &str {
            &self.model
        }

        async fn stream(
            &self,
            _system: Option<&str>,
            _messages: Vec<Message>,
            _tools: Vec<serde_json::Value>,
        ) -> Result<Box<dyn LlmStream + Send + Unpin>> {
            let events = self.events.lock().unwrap().clone();
            Ok(Box::new(MockStream::new(events)))
        }
    }

    fn create_test_agent(events: Vec<StreamEvent>) -> Agent {
        let provider = MockProvider::new("test-model".to_string(), events);
        let tools = ToolRegistry::new();
        let working_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        Agent::new(Box::new(provider), tools, working_dir)
    }

    #[tokio::test]
    async fn test_agent_new() {
        let events = vec![StreamEvent::MessageStop];
        let agent = create_test_agent(events);

        assert_eq!(agent.messages.len(), 0);
        assert!(!agent.system_prompt.is_empty());
    }

    #[tokio::test]
    async fn test_agent_set_system_prompt() {
        let events = vec![StreamEvent::MessageStop];
        let mut agent = create_test_agent(events);

        agent.set_system_prompt("Custom prompt".to_string());
        assert_eq!(agent.system_prompt, "Custom prompt");
    }

    #[tokio::test]
    async fn test_agent_send_text_response() {
        let events = vec![
            StreamEvent::ContentBlockStart {
                index: 0,
                content_block: ContentBlock::Text {
                    text: "Hello, world!".to_string(),
                },
            },
            StreamEvent::ContentBlockStop { index: 0 },
            StreamEvent::MessageStop,
        ];

        let mut agent = create_test_agent(events);
        let (tx, mut rx) = mpsc::unbounded_channel();

        agent.send("Hi".to_string(), tx).await.unwrap();

        let mut received_text = false;
        let mut received_complete = false;

        while let Ok(event) = rx.try_recv() {
            match event {
                AgentEvent::TextDelta { text } => {
                    assert_eq!(text, "Hello, world!");
                    received_text = true;
                }
                AgentEvent::Complete => received_complete = true,
                _ => {}
            }
        }

        assert!(received_text);
        assert!(received_complete);
        assert_eq!(agent.messages.len(), 2);
    }

    #[tokio::test]
    async fn test_agent_send_with_thinking() {
        let events = vec![
            StreamEvent::ContentBlockStart {
                index: 0,
                content_block: ContentBlock::Thinking {
                    thinking: "Let me think...".to_string(),
                },
            },
            StreamEvent::ContentBlockStop { index: 0 },
            StreamEvent::ContentBlockStart {
                index: 1,
                content_block: ContentBlock::Text {
                    text: "Response".to_string(),
                },
            },
            StreamEvent::ContentBlockStop { index: 1 },
            StreamEvent::MessageStop,
        ];

        let mut agent = create_test_agent(events);
        let (tx, mut rx) = mpsc::unbounded_channel();

        agent.send("Hi".to_string(), tx).await.unwrap();

        let mut received_thinking = false;
        let mut received_text = false;

        while let Ok(event) = rx.try_recv() {
            match event {
                AgentEvent::ThinkingDelta { thinking } => {
                    assert_eq!(thinking, "Let me think...");
                    received_thinking = true;
                }
                AgentEvent::TextDelta { text } => {
                    assert_eq!(text, "Response");
                    received_text = true;
                }
                _ => {}
            }
        }

        assert!(received_thinking);
        assert!(received_text);
    }

    #[tokio::test]
    async fn test_agent_token_usage() {
        let events = vec![
            StreamEvent::ContentBlockStart {
                index: 0,
                content_block: ContentBlock::Text {
                    text: "Response".to_string(),
                },
            },
            StreamEvent::ContentBlockStop { index: 0 },
            StreamEvent::MessageDelta {
                delta: catalyst_llm::MessageDeltaInfo {
                    stop_reason: Some("end_turn".to_string()),
                },
                usage: Usage {
                    input_tokens: 100,
                    output_tokens: 50,
                    cache_creation_input_tokens: 0,
                    cache_read_input_tokens: 0,
                },
            },
            StreamEvent::MessageStop,
        ];

        let mut agent = create_test_agent(events);
        let (tx, mut rx) = mpsc::unbounded_channel();

        agent.send("Hi".to_string(), tx).await.unwrap();

        let mut received_usage = false;

        while let Ok(event) = rx.try_recv() {
            if let AgentEvent::TokenUsage { input, output } = event {
                assert_eq!(input, 100);
                assert_eq!(output, 50);
                received_usage = true;
            }
        }

        assert!(received_usage);
    }

    #[tokio::test]
    async fn test_agent_error_handling() {
        let events = vec![StreamEvent::Error {
            error: catalyst_llm::ApiError {
                error_type: "rate_limit".to_string(),
                message: "Rate limit exceeded".to_string(),
            },
        }];

        let mut agent = create_test_agent(events);
        let (tx, mut rx) = mpsc::unbounded_channel();

        agent.send("Hi".to_string(), tx).await.unwrap();

        let mut received_error = false;

        while let Ok(event) = rx.try_recv() {
            if let AgentEvent::Error(msg) = event {
                assert!(msg.contains("Rate limit exceeded"));
                received_error = true;
            }
        }

        assert!(received_error);
    }

    #[tokio::test]
    async fn test_agent_user_message_added() {
        let events = vec![StreamEvent::MessageStop];
        let mut agent = create_test_agent(events);
        let (tx, _) = mpsc::unbounded_channel();

        agent.send("Test message".to_string(), tx).await.unwrap();

        assert_eq!(agent.messages.len(), 1);
        match &agent.messages[0] {
            Message {
                role: Role::User,
                content: Content::Text(text),
            } => assert_eq!(text, "Test message"),
            _ => panic!("Expected user message"),
        }
    }

    #[tokio::test]
    async fn test_agent_conversation_history() {
        let events1 = vec![
            StreamEvent::ContentBlockStart {
                index: 0,
                content_block: ContentBlock::Text {
                    text: "First response".to_string(),
                },
            },
            StreamEvent::ContentBlockStop { index: 0 },
            StreamEvent::MessageStop,
        ];

        let mut agent = create_test_agent(events1);
        let (tx, _) = mpsc::unbounded_channel();

        agent.send("First".to_string(), tx.clone()).await.unwrap();
        assert_eq!(agent.messages.len(), 2);

        agent.messages.clear();

        let events2 = vec![
            StreamEvent::ContentBlockStart {
                index: 0,
                content_block: ContentBlock::Text {
                    text: "Second response".to_string(),
                },
            },
            StreamEvent::ContentBlockStop { index: 0 },
            StreamEvent::MessageStop,
        ];

        let provider2 = MockProvider::new("test-model".to_string(), events2);
        agent.provider = Box::new(provider2);

        agent.send("Second".to_string(), tx).await.unwrap();
        assert_eq!(agent.messages.len(), 2);
    }
}
