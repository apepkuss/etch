use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

/// 会话状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SessionStatus {
    Active,
    Completed,
    Failed,
    Timeout,
}

/// 会话信息
#[derive(Debug, Clone, Serialize)]
pub struct SessionInfo {
    pub session_id: String,
    pub device_id: String,
    pub echokit_session_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub status: SessionStatus,
    pub audio_frames_sent: u64,
    pub audio_frames_received: u64,
}

/// 会话管理器
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<String, SessionInfo>>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 创建会话
    pub async fn create_session(
        &self,
        session_id: String,
        device_id: String,
    ) -> anyhow::Result<()> {
        let session_info = SessionInfo {
            session_id: session_id.clone(),
            device_id: device_id.clone(),
            echokit_session_id: None,
            created_at: Utc::now(),
            last_activity: Utc::now(),
            status: SessionStatus::Active,
            audio_frames_sent: 0,
            audio_frames_received: 0,
        };

        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id.clone(), session_info);

        info!("Session {} created for device {}", session_id, device_id);
        Ok(())
    }

    /// 更新会话活动时间
    pub async fn update_activity(&self, session_id: &str) -> anyhow::Result<()> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.last_activity = Utc::now();
        }
        Ok(())
    }

    /// 增加发送帧计数
    pub async fn increment_sent_frames(&self, session_id: &str) {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.audio_frames_sent += 1;
            session.last_activity = Utc::now();
        }
    }

    /// 增加接收帧计数
    pub async fn increment_received_frames(&self, session_id: &str) {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.audio_frames_received += 1;
            session.last_activity = Utc::now();
        }
    }

    /// 结束会话
    pub async fn end_session(&self, session_id: &str) -> anyhow::Result<()> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.status = SessionStatus::Completed;
            info!("Session {} ended (sent: {}, received: {})",
                  session_id, session.audio_frames_sent, session.audio_frames_received);
        }
        Ok(())
    }

    /// 标记会话失败
    pub async fn mark_failed(&self, session_id: &str) -> anyhow::Result<()> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.status = SessionStatus::Failed;
        }
        Ok(())
    }

    /// 获取会话信息
    pub async fn get_session(&self, session_id: &str) -> Option<SessionInfo> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).cloned()
    }

    /// 获取设备的所有活跃会话
    pub async fn get_device_sessions(&self, device_id: &str) -> Vec<SessionInfo> {
        let sessions = self.sessions.read().await;
        sessions
            .values()
            .filter(|s| s.device_id == device_id && s.status == SessionStatus::Active)
            .cloned()
            .collect()
    }

    /// 获取设备的所有会话ID
    pub async fn get_sessions_by_device(&self, device_id: &str) -> Vec<String> {
        let sessions = self.sessions.read().await;
        sessions
            .values()
            .filter(|s| s.device_id == device_id)
            .map(|s| s.session_id.clone())
            .collect()
    }

    /// 标记会话为超时
    pub async fn mark_timeout(&self, session_id: &str) -> anyhow::Result<()> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.status = SessionStatus::Timeout;
            info!("Session {} marked as timeout", session_id);
        }
        Ok(())
    }

    /// 清理超时会话
    pub async fn cleanup_timeout_sessions(&self, timeout_seconds: i64) -> usize {
        let now = Utc::now();
        let mut sessions = self.sessions.write().await;

        let mut timeout_sessions = Vec::new();
        for (session_id, session) in sessions.iter_mut() {
            if session.status == SessionStatus::Active {
                let duration = now.signed_duration_since(session.last_activity);
                if duration.num_seconds() > timeout_seconds {
                    session.status = SessionStatus::Timeout;
                    timeout_sessions.push(session_id.clone());
                }
            }
        }

        let count = timeout_sessions.len();
        if count > 0 {
            info!("Cleaned up {} timeout sessions", count);
        }

        count
    }

    /// 获取统计信息
    pub async fn get_stats(&self) -> SessionStats {
        let sessions = self.sessions.read().await;

        let mut stats = SessionStats {
            total: sessions.len(),
            active: 0,
            completed: 0,
            failed: 0,
            timeout: 0,
        };

        for session in sessions.values() {
            match session.status {
                SessionStatus::Active => stats.active += 1,
                SessionStatus::Completed => stats.completed += 1,
                SessionStatus::Failed => stats.failed += 1,
                SessionStatus::Timeout => stats.timeout += 1,
            }
        }

        stats
    }
}

/// 会话统计
#[derive(Debug, Serialize)]
pub struct SessionStats {
    pub total: usize,
    pub active: usize,
    pub completed: usize,
    pub failed: usize,
    pub timeout: usize,
}
