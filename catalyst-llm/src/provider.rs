use crate::types::{Message, StreamEvent};
use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait LlmProvider: Send + Sync {
    fn name(&self) -> &str;
    fn model(&self) -> &str;

    async fn stream(
        &self,
        system: Option<&str>,
        messages: Vec<Message>,
        tools: Vec<serde_json::Value>,
    ) -> Result<Box<dyn LlmStream + Send + Unpin>>;
}

#[async_trait]
pub trait LlmStream {
    async fn next_event(&mut self) -> Result<Option<StreamEvent>>;
}
