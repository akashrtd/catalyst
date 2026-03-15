use crate::types::*;
use anyhow::Result;
use futures::stream::Stream;
use reqwest::Client;
use std::pin::Pin;
use std::task::{Context as TaskContext, Poll};
use tracing::debug;

pub struct AnthropicClient {
    http: Client,
    api_key: String,
    model: String,
    base_url: String,
}

impl AnthropicClient {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            http: Client::new(),
            api_key,
            model,
            base_url: "https://api.anthropic.com".to_string(),
        }
    }
    
    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.base_url = base_url;
        self
    }
    
    pub async fn stream(
        &self,
        system: Option<&str>,
        messages: Vec<Message>,
        tools: Vec<serde_json::Value>,
    ) -> Result<SseStream> {
        let request = MessageRequest {
            model: self.model.clone(),
            max_tokens: 4096,
            system: system.map(|s| s.to_string()),
            messages,
            tools: tools.iter().filter_map(|v| {
                serde_json::from_value::<ToolDef>(v.clone()).ok()
            }).collect(),
            stream: true,
        };
        
        let response = self.http
            .post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .header("accept", "text/event-stream")
            .json(&request)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send request: {}", e))?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("API error ({}): {}", status, body);
        }
        
        Ok(SseStream::new(response))
    }
}

pub struct SseStream {
    response: Pin<Box<dyn Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send>>,
    buffer: String,
}

impl SseStream {
    pub fn new(response: reqwest::Response) -> Self {
        Self {
            response: Box::pin(response.bytes_stream()),
            buffer: String::new(),
        }
    }
    
    pub async fn next_event(&mut self) -> Result<Option<StreamEvent>> {
        use futures::StreamExt;
        
        loop {
            while let Some(pos) = self.buffer.find("\n\n") {
                let event_str = self.buffer.drain(..pos + 2).collect::<String>();
                
                for line in event_str.lines() {
                    if let Some(json_str) = line.strip_prefix("data: ") {
                        if json_str.is_empty() {
                            continue;
                        }
                        
                        match serde_json::from_str::<StreamEvent>(json_str) {
                            Ok(event) => return Ok(Some(event)),
                            Err(e) => {
                                debug!("Failed to parse event: {} - {}", e, json_str);
                                continue;
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

impl Stream for SseStream {
    type Item = Result<StreamEvent>;
    
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<Option<Self::Item>> {
        while let Some(pos) = self.buffer.find("\n\n") {
            let event_str = self.buffer.drain(..pos + 2).collect::<String>();
            
            for line in event_str.lines() {
                if let Some(json_str) = line.strip_prefix("data: ") {
                    if json_str.is_empty() {
                        continue;
                    }
                    
                    match serde_json::from_str::<StreamEvent>(json_str) {
                        Ok(event) => return Poll::Ready(Some(Ok(event))),
                        Err(e) => {
                            debug!("Failed to parse event: {} - {}", e, json_str);
                            continue;
                        }
                    }
                }
            }
        }
        
        match self.response.as_mut().poll_next(cx) {
            Poll::Ready(Some(Ok(chunk))) => {
                self.buffer.push_str(&String::from_utf8_lossy(&chunk));
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(anyhow::anyhow!("Stream error: {}", e)))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}
