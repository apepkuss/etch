use echo_shared::{Session, SessionStatus};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;

// 会话管理器
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<String, Session>>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn create_session(&self, device_id: &str, user_id: &str) -> Session {
        let session = Session {
            id: Uuid::new_v4().to_string(),
            device_id: device_id.to_string(),
            user_id: user_id.to_string(),
            start_time: chrono::Utc::now(),
            end_time: None,
            duration: None,
            transcription: None,
            response: None,
            status: SessionStatus::Active,
        };

        let mut sessions = self.sessions.write().await;
        sessions.insert(session.id.clone(), session.clone());

        info!("Created new session: {} for device: {}", session.id, device_id);
        session
    }

    pub async fn update_session(&self, session_id: &str, updates: SessionUpdate) -> Option<Session> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            apply_updates(session, updates);
            info!("Updated session: {}", session_id);
            Some(session.clone())
        } else {
            warn!("Session not found: {}", session_id);
            None
        }
    }

    pub async fn get_session(&self, session_id: &str) -> Option<Session> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).cloned()
    }

    pub async fn complete_session(&self, session_id: &str, transcription: String, response: String) {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.end_time = Some(chrono::Utc::now());
            session.transcription = Some(transcription);
            session.response = Some(response);
            session.status = SessionStatus::Completed;

            let start_time = session.start_time.timestamp();
            if let Some(end_time) = session.end_time {
                let end_timestamp = end_time.timestamp();
                session.duration = Some((end_timestamp - start_time) as i32);
            }

            info!("Completed session: {}", session_id);
        }
    }

    pub async fn list_active_sessions(&self) -> Vec<Session> {
        let sessions = self.sessions.read().await;
        sessions
            .values()
            .filter(|s| matches!(s.status, SessionStatus::Active))
            .cloned()
            .collect()
    }
}

// 会话更新结构
pub struct SessionUpdate {
    pub transcription: Option<String>,
    pub response: Option<String>,
    pub status: Option<SessionStatus>,
}

fn apply_updates(session: &mut Session, updates: SessionUpdate) {
    if let Some(transcription) = updates.transcription {
        session.transcription = Some(transcription);
    }
    if let Some(response) = updates.response {
        session.response = Some(response);
    }
    if let Some(status) = updates.status {
        session.status = status;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_lifecycle() {
        let manager = SessionManager::new();

        // 创建会话
        let session = manager.create_session("dev001", "user001").await;
        assert_eq!(session.status, SessionStatus::Active);
        assert!(session.transcription.is_none());

        // 更新会话
        let updates = SessionUpdate {
            transcription: Some("Hello world".to_string()),
            response: Some("Hi there!".to_string()),
            status: None,
        };

        let updated = manager.update_session(&session.id, updates).await;
        assert!(updated.is_some());

        let updated_session = updated.unwrap();
        assert_eq!(updated_session.transcription, Some("Hello world".to_string()));
        assert_eq!(updated_session.response, Some("Hi there!".to_string()));

        // 完成会话
        manager.complete_session(&session.id,
            "Final transcription".to_string(),
            "Final response".to_string()
        ).await;

        let completed = manager.get_session(&session.id).await;
        assert!(completed.is_some());
        assert_eq!(completed.unwrap().status, SessionStatus::Completed);
    }
}