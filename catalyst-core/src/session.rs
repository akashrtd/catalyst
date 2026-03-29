use anyhow::Result;
use catalyst_llm::{Message, Role};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    pub id: String,
    pub model: String,
    pub provider: String,
    pub messages: Vec<Message>,
    pub created_at: u64,
    pub updated_at: u64,
}

impl SessionData {
    pub fn new(model: String, provider: String) -> Self {
        let now = system_time_secs();
        Self {
            id: uuid::Uuid::new_v4().to_string()[..8].to_string(),
            model,
            provider,
            messages: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    pub fn user_message_count(&self) -> usize {
        self.messages
            .iter()
            .filter(|m| matches!(m.role, Role::User))
            .count()
    }

    pub fn preview(&self) -> String {
        self.messages
            .iter()
            .find(|m| matches!(m.role, Role::User))
            .map(|m| match &m.content {
                catalyst_llm::Content::Text(t) => t.chars().take(80).collect(),
                catalyst_llm::Content::Blocks(blocks) => blocks
                    .iter()
                    .filter_map(|b| match b {
                        catalyst_llm::ContentBlock::Text { text } => Some(text.as_str()),
                        _ => None,
                    })
                    .next()
                    .unwrap_or("")
                    .chars()
                    .take(80)
                    .collect(),
            })
            .unwrap_or_default()
    }
}

fn system_time_secs() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn sessions_dir() -> Result<PathBuf> {
    let dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("catalyst")
        .join("sessions");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn save_session(session: &mut SessionData) -> Result<()> {
    let dir = sessions_dir()?;
    session.updated_at = system_time_secs();
    let path = dir.join(format!("{}.json", session.id));
    let json = serde_json::to_string_pretty(session)?;
    std::fs::write(path, json)?;
    Ok(())
}

pub fn load_session(id: &str) -> Result<SessionData> {
    let dir = sessions_dir()?;
    let path = dir.join(format!("{}.json", id));
    let json = std::fs::read_to_string(path)?;
    let session: SessionData = serde_json::from_str(&json)?;
    Ok(session)
}

pub fn list_sessions() -> Result<Vec<SessionData>> {
    let dir = sessions_dir()?;
    let mut sessions = Vec::new();

    if !dir.exists() {
        return Ok(sessions);
    }

    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            if let Ok(json) = std::fs::read_to_string(&path) {
                if let Ok(session) = serde_json::from_str::<SessionData>(&json) {
                    sessions.push(session);
                }
            }
        }
    }

    sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(sessions)
}

pub fn delete_session(id: &str) -> Result<()> {
    let dir = sessions_dir()?;
    let path = dir.join(format!("{}.json", id));
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_data_new() {
        let session = SessionData::new("claude-sonnet-4".to_string(), "anthropic".to_string());
        assert_eq!(session.model, "claude-sonnet-4");
        assert_eq!(session.provider, "anthropic");
        assert_eq!(session.id.len(), 8);
        assert!(session.messages.is_empty());
        assert!(session.created_at > 0);
        assert_eq!(session.created_at, session.updated_at);
    }

    #[test]
    fn test_session_data_message_count() {
        let mut session = SessionData::new("model".to_string(), "provider".to_string());
        assert_eq!(session.message_count(), 0);
        assert_eq!(session.user_message_count(), 0);

        session.messages.push(Message {
            role: Role::User,
            content: catalyst_llm::Content::Text("Hello".to_string()),
        });
        session.messages.push(Message {
            role: Role::Assistant,
            content: catalyst_llm::Content::Text("Hi".to_string()),
        });
        assert_eq!(session.message_count(), 2);
        assert_eq!(session.user_message_count(), 1);
    }

    #[test]
    fn test_session_data_preview() {
        let mut session = SessionData::new("model".to_string(), "provider".to_string());
        assert!(session.preview().is_empty());

        session.messages.push(Message {
            role: Role::User,
            content: catalyst_llm::Content::Text(
                "Fix the auth bug in the login handler".to_string(),
            ),
        });
        assert_eq!(session.preview(), "Fix the auth bug in the login handler");
    }

    #[test]
    fn test_session_data_preview_long_message() {
        let mut session = SessionData::new("model".to_string(), "provider".to_string());
        let long_msg = "x".repeat(200);
        session.messages.push(Message {
            role: Role::User,
            content: catalyst_llm::Content::Text(long_msg),
        });
        assert_eq!(session.preview().len(), 80);
    }

    #[test]
    fn test_session_data_preview_blocks() {
        let mut session = SessionData::new("model".to_string(), "provider".to_string());
        session.messages.push(Message {
            role: Role::User,
            content: catalyst_llm::Content::Blocks(vec![catalyst_llm::ContentBlock::Text {
                text: "Block message".to_string(),
            }]),
        });
        assert_eq!(session.preview(), "Block message");
    }

    #[test]
    fn test_session_serialization_roundtrip() {
        let mut session = SessionData::new("test-model".to_string(), "anthropic".to_string());
        session.messages.push(Message {
            role: Role::User,
            content: catalyst_llm::Content::Text("Hello".to_string()),
        });
        session.messages.push(Message {
            role: Role::Assistant,
            content: catalyst_llm::Content::Blocks(vec![
                catalyst_llm::ContentBlock::Text {
                    text: "Response".to_string(),
                },
                catalyst_llm::ContentBlock::ToolUse {
                    id: "tool_1".to_string(),
                    name: "read".to_string(),
                    input: serde_json::json!({"path": "/test.rs"}),
                },
            ]),
        });

        let json = serde_json::to_string(&session).unwrap();
        let deserialized: SessionData = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, session.id);
        assert_eq!(deserialized.model, session.model);
        assert_eq!(deserialized.messages.len(), 2);
        assert_eq!(deserialized.user_message_count(), 1);
    }

    #[test]
    fn test_session_save_and_load() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let sessions_path = tmp_dir.path().join("sessions");
        std::fs::create_dir_all(&sessions_path).unwrap();

        let mut session = SessionData::new("test-model".to_string(), "anthropic".to_string());
        session.messages.push(Message {
            role: Role::User,
            content: catalyst_llm::Content::Text("Test message".to_string()),
        });

        let path = sessions_path.join(format!("{}.json", session.id));
        let json = serde_json::to_string_pretty(&session).unwrap();
        std::fs::write(&path, &json).unwrap();

        let loaded_json = std::fs::read_to_string(&path).unwrap();
        let loaded: SessionData = serde_json::from_str(&loaded_json).unwrap();

        assert_eq!(loaded.id, session.id);
        assert_eq!(loaded.messages.len(), 1);
    }
}
