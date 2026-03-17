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

#[cfg(test)]
pub mod mock {
    use super::*;
    use std::sync::{Arc, Mutex};

    pub struct MockStream {
        events: Arc<Mutex<Vec<StreamEvent>>>,
    }

    impl MockStream {
        pub fn new(events: Vec<StreamEvent>) -> Self {
            Self {
                events: Arc::new(Mutex::new(events)),
            }
        }
    }

    #[async_trait]
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

    pub struct MockProvider {
        events: Arc<Mutex<Vec<StreamEvent>>>,
        model: String,
    }

    impl MockProvider {
        pub fn new(model: String, events: Vec<StreamEvent>) -> Self {
            Self {
                events: Arc::new(Mutex::new(events)),
                model,
            }
        }

        pub fn with_text_response(model: &str, text: &str) -> Self {
            Self::new(
                model.to_string(),
                vec![
                    StreamEvent::ContentBlockStart {
                        index: 0,
                        content_block: crate::types::ContentBlock::Text {
                            text: text.to_string(),
                        },
                    },
                    StreamEvent::MessageStop,
                ],
            )
        }
    }

    #[async_trait]
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

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::types::{ContentBlock, Role, Usage};

        #[tokio::test]
        async fn test_mock_provider() {
            let provider = MockProvider::with_text_response("test-model", "Hello, world!");
            let stream = provider.stream(None, vec![], vec![]).await.unwrap();
            
            assert_eq!(provider.name(), "mock");
            assert_eq!(provider.model(), "test-model");
        }

        #[tokio::test]
        async fn test_mock_stream_events() {
            let events = vec![
                StreamEvent::MessageStart {
                    message: crate::types::MessageInfo {
                        id: "msg_123".to_string(),
                        model: "test-model".to_string(),
                        role: Role::Assistant,
                        content: vec![],
                    },
                },
                StreamEvent::ContentBlockStart {
                    index: 0,
                    content_block: ContentBlock::Text {
                        text: "Hello".to_string(),
                    },
                },
                StreamEvent::MessageDelta {
                    delta: crate::types::MessageDeltaInfo {
                        stop_reason: Some("end_turn".to_string()),
                    },
                    usage: Usage {
                        input_tokens: 10,
                        output_tokens: 5,
                        cache_creation_input_tokens: 0,
                        cache_read_input_tokens: 0,
                    },
                },
                StreamEvent::MessageStop,
            ];

            let provider = MockProvider::new("test-model".to_string(), events);
            let mut stream = provider.stream(None, vec![], vec![]).await.unwrap();

            let event = stream.next_event().await.unwrap();
            assert!(matches!(event, Some(StreamEvent::MessageStart { .. })));

            let event = stream.next_event().await.unwrap();
            assert!(matches!(event, Some(StreamEvent::ContentBlockStart { .. })));

            let event = stream.next_event().await.unwrap();
            assert!(matches!(event, Some(StreamEvent::MessageDelta { .. })));

            let event = stream.next_event().await.unwrap();
            assert!(matches!(event, Some(StreamEvent::MessageStop)));

            let event = stream.next_event().await.unwrap();
            assert!(event.is_none());
        }
    }
}
