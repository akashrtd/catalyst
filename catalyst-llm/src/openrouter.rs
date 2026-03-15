use crate::provider::{LlmProvider, LlmStream};
use crate::types::*;
use anyhow::Result;
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tracing::debug;

pub struct OpenRouterClient {
    http: Client,
    api_key: String,
    model: String,
    base_url: String,
}

impl OpenRouterClient {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            http: Client::new(),
            api_key,
            model,
            base_url: "https://openrouter.ai/api/v1".to_string(),
        }
    }
}

#[async_trait]
impl LlmProvider for OpenRouterClient {
    fn name(&self) -> &str {
        "openrouter"
    }

    fn model(&self) -> &str {
        &self.model
    }

    async fn stream(
        &self,
        system: Option<&str>,
        messages: Vec<Message>,
        _tools: Vec<serde_json::Value>,
    ) -> Result<Box<dyn LlmStream + Send + Unpin>> {
        let mut openai_messages: Vec<OpenAiMessage> = Vec::new();

        if let Some(sys) = system {
            openai_messages.push(OpenAiMessage {
                role: "system".to_string(),
                content: Some(sys.to_string()),
                tool_calls: None,
            });
        }

        for msg in messages {
            match &msg.content {
                Content::Text(text) => {
                    openai_messages.push(OpenAiMessage {
                        role: match msg.role {
                            Role::User => "user",
                            Role::Assistant => "assistant",
                        }
                        .to_string(),
                        content: Some(text.clone()),
                        tool_calls: None,
                    });
                }
                Content::Blocks(blocks) => {
                    for block in blocks {
                        match block {
                            ContentBlock::Text { text } => {
                                openai_messages.push(OpenAiMessage {
                                    role: match msg.role {
                                        Role::User => "user",
                                        Role::Assistant => "assistant",
                                    }
                                    .to_string(),
                                    content: Some(text.clone()),
                                    tool_calls: None,
                                });
                            }
                            ContentBlock::Thinking { thinking } => {
                                openai_messages.push(OpenAiMessage {
                                    role: "assistant".to_string(),
                                    content: Some(format!("[thinking] {}", thinking)),
                                    tool_calls: None,
                                });
                            }
                            ContentBlock::ToolUse { id, name, input } => {
                                openai_messages.push(OpenAiMessage {
                                    role: "assistant".to_string(),
                                    content: None,
                                    tool_calls: Some(vec![OpenAiToolCall {
                                        id: id.clone(),
                                        r#type: "function".to_string(),
                                        function: FunctionCall {
                                            name: name.clone(),
                                            arguments: serde_json::to_string(input)
                                                .unwrap_or_default(),
                                        },
                                    }]),
                                });
                            }
                            ContentBlock::ToolResult {
                                tool_use_id: _,
                                content,
                                ..
                            } => {
                                openai_messages.push(OpenAiMessage {
                                    role: "tool".to_string(),
                                    content: Some(content.clone()),
                                    tool_calls: None,
                                });
                            }
                            ContentBlock::Redacted { text } => {
                                openai_messages.push(OpenAiMessage {
                                    role: match msg.role {
                                        Role::User => "user",
                                        Role::Assistant => "assistant",
                                    }
                                    .to_string(),
                                    content: Some(format!("[redacted] {}", text)),
                                    tool_calls: None,
                                });
                            }
                        }
                    }
                }
            }
        }

        let request = OpenAiRequest {
            model: self.model.clone(),
            messages: openai_messages,
            stream: true,
        };

        let response = self
            .http
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .header("Accept", "text/event-stream")
            .json(&request)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send request: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("OpenRouter API error ({}): {}", status, body);
        }

        Ok(Box::new(OpenRouterStream::new(response)))
    }
}

pub struct OpenRouterStream {
    response:
        std::pin::Pin<Box<dyn futures::Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send>>,
    buffer: String,
    tool_call_buffers: HashMap<String, (String, String)>,
}

impl OpenRouterStream {
    pub fn new(response: reqwest::Response) -> Self {
        Self {
            response: Box::pin(response.bytes_stream()),
            buffer: String::new(),
            tool_call_buffers: HashMap::new(),
        }
    }
}

#[async_trait]
impl LlmStream for OpenRouterStream {
    async fn next_event(&mut self) -> Result<Option<StreamEvent>> {
        loop {
            while let Some(pos) = self.buffer.find("\n\n") {
                let event_str = self.buffer.drain(..pos + 2).collect::<String>();

                for line in event_str.lines() {
                    if let Some(json_str) = line.strip_prefix("data: ") {
                        if json_str == "[DONE]" {
                            return Ok(None);
                        }

                        if json_str.is_empty() {
                            continue;
                        }

                        match serde_json::from_str::<OpenAiStreamResponse>(json_str) {
                            Ok(resp) => {
                                if let Some(event) = self.convert_event(resp) {
                                    return Ok(Some(event));
                                }
                            }
                            Err(e) => {
                                debug!("Failed to parse OpenRouter event: {} - {}", e, json_str);
                            }
                        }
                    }
                }
            }

            match self.response.next().await {
                Some(Ok(chunk)) => {
                    self.buffer.push_str(&String::from_utf8_lossy(&chunk));
                }
                Some(Err(e)) => anyhow::bail!("Stream error: {}", e),
                None => return Ok(None),
            }
        }
    }
}

impl OpenRouterStream {
    fn convert_event(&mut self, resp: OpenAiStreamResponse) -> Option<StreamEvent> {
        let choice = resp.choices.first()?;

        if let Some(finish_reason) = &choice.finish_reason {
            if finish_reason == "stop" {
                return Some(StreamEvent::MessageStop);
            }
        }

        let delta = &choice.delta;

        if let Some(role) = &delta.role {
            if role == "assistant" {
                return Some(StreamEvent::MessageStart {
                    message: MessageInfo {
                        id: resp.id.clone(),
                        model: resp.model.clone(),
                        role: Role::Assistant,
                        content: vec![],
                    },
                });
            }
        }

        if let Some(tool_calls) = &delta.tool_calls {
            if let Some(tc) = tool_calls.iter().next() {
                let id = tc.id.clone().unwrap_or_default();
                let name = tc.function.name.clone().unwrap_or_default();
                let index = tc.index.unwrap_or(0);
                self.tool_call_buffers
                    .insert(id.clone(), (name.clone(), String::new()));

                return Some(StreamEvent::ContentBlockStart {
                    index,
                    content_block: ContentBlock::ToolUse {
                        id,
                        name,
                        input: Value::Object(serde_json::Map::new()),
                    },
                });
            }
        }

        if let Some(content) = &delta.content {
            if !content.is_empty() {
                return Some(StreamEvent::ContentBlockDelta {
                    index: 0,
                    delta: Delta::TextDelta {
                        text: content.clone(),
                    },
                });
            }
        }

        if let Some(tool_calls) = &delta.tool_calls {
            for tc in tool_calls {
                if let (Some(id), Some(args_delta)) = (&tc.id, &tc.function.arguments) {
                    if let Some((_, args)) = self.tool_call_buffers.get_mut(id) {
                        args.push_str(args_delta);
                        return Some(StreamEvent::ContentBlockDelta {
                            index: tc.index.unwrap_or(0),
                            delta: Delta::InputJsonDelta {
                                partial_json: args_delta.to_string(),
                            },
                        });
                    }
                }
            }
        }

        None
    }
}

#[derive(Debug, Serialize)]
struct OpenAiRequest {
    model: String,
    messages: Vec<OpenAiMessage>,
    stream: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAiMessage {
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<OpenAiToolCall>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAiToolCall {
    id: String,
    r#type: String,
    function: FunctionCall,
}

#[derive(Debug, Serialize, Deserialize)]
struct FunctionCall {
    name: String,
    arguments: String,
}

#[derive(Debug, Deserialize)]
struct OpenAiStreamResponse {
    id: String,
    model: String,
    choices: Vec<OpenAiChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    #[allow(dead_code)]
    index: usize,
    delta: OpenAiDelta,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAiDelta {
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<OpenAiDeltaToolCall>>,
}

#[derive(Debug, Deserialize)]
struct OpenAiDeltaToolCall {
    index: Option<usize>,
    id: Option<String>,
    function: OpenAiDeltaFunction,
}

#[derive(Debug, Deserialize)]
struct OpenAiDeltaFunction {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    arguments: Option<String>,
}
