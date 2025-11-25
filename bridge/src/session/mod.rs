use echo_shared::Session;
use echo_shared::types::SessionStatus;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error};
use uuid::Uuid;
use sqlx::PgPool;
use anyhow::Result;
use chrono::Utc;

// 会话管理器
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<String, Session>>>,
    db_pool: PgPool,
}

impl SessionManager {
    pub fn new(db_pool: PgPool) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            db_pool,
        }
    }

    /// 创建会话 -> 同时写入数据库
    pub async fn create_session(
        &self,
        device_id: &str,
        user_id: &str
    ) -> Result<Session> {
        let session = Session {
            id: Uuid::new_v4().to_string(),
            device_id: device_id.to_string(),
            user_id: Some(user_id.to_string()), // 🔧 修复：user_id 现在是 Option<String>
            start_time: Utc::now(),
            end_time: None,
            duration: None,
            transcription: None,
            response: None,
            status: SessionStatus::Active,
        };

        // 写入数据库
        sqlx::query!(
            r#"
            INSERT INTO sessions (id, device_id, user_id, start_time, status)
            VALUES ($1, $2, $3, $4, $5)
            "#,
            session.id,
            session.device_id,
            session.user_id,
            session.start_time,
            "active"
        )
        .execute(&self.db_pool)
        .await
        .map_err(|e| {
            error!("Failed to insert session into database: {}", e);
            anyhow::anyhow!("Database insert failed: {}", e)
        })?;

        // 同时保存到内存（用于快速访问活跃会话）
        let mut sessions = self.sessions.write().await;
        sessions.insert(session.id.clone(), session.clone());

        info!("Created session {} and saved to DB", session.id);
        Ok(session)
    }

    /// 更新会话转录文本
    pub async fn update_transcription(
        &self,
        session_id: &str,
        transcription: String
    ) -> Result<()> {
        // 更新数据库
        sqlx::query!(
            r#"
            UPDATE sessions
            SET transcription = $1
            WHERE id = $2
            "#,
            transcription,
            session_id
        )
        .execute(&self.db_pool)
        .await
        .map_err(|e| {
            error!("Failed to update transcription for session {}: {}", session_id, e);
            anyhow::anyhow!("Database update failed: {}", e)
        })?;

        // 更新内存
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.transcription = Some(transcription);
        }

        info!("Updated transcription for session {}", session_id);
        Ok(())
    }

    /// 完成会话 -> 更新数据库
    pub async fn complete_session(
        &self,
        session_id: &str,
        transcription: String,
        response: String
    ) -> Result<()> {
        let now = Utc::now();

        // 更新数据库
        sqlx::query!(
            r#"
            UPDATE sessions
            SET
                end_time = $1,
                transcription = $2,
                response = $3,
                status = $4,
                duration = EXTRACT(EPOCH FROM ($1 - start_time))::INTEGER
            WHERE id = $5
            "#,
            now,
            transcription,
            response,
            "completed",
            session_id
        )
        .execute(&self.db_pool)
        .await
        .map_err(|e| {
            error!("Failed to complete session {}: {}", session_id, e);
            anyhow::anyhow!("Database update failed: {}", e)
        })?;

        // 更新内存
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.end_time = Some(now);
            session.transcription = Some(transcription);
            session.response = Some(response);
            session.status = SessionStatus::Completed;

            if let Some(end_time) = session.end_time {
                let duration = end_time.signed_duration_since(session.start_time);
                session.duration = Some(duration.num_seconds() as i32);
            }
        }

        info!("Completed session {} and updated DB", session_id);
        Ok(())
    }

    /// 标记会话失败
    pub async fn fail_session(
        &self,
        session_id: &str,
        error_message: &str
    ) -> Result<()> {
        let now = Utc::now();

        // 更新数据库
        sqlx::query!(
            r#"
            UPDATE sessions
            SET
                end_time = $1,
                status = $2,
                response = $3,
                duration = EXTRACT(EPOCH FROM ($1 - start_time))::INTEGER
            WHERE id = $4
            "#,
            now,
            "failed",
            error_message,
            session_id
        )
        .execute(&self.db_pool)
        .await
        .map_err(|e| {
            error!("Failed to mark session {} as failed: {}", session_id, e);
            anyhow::anyhow!("Database update failed: {}", e)
        })?;

        // 更新内存
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.end_time = Some(now);
            session.status = SessionStatus::Failed;
            session.response = Some(error_message.to_string());

            if let Some(end_time) = session.end_time {
                let duration = end_time.signed_duration_since(session.start_time);
                session.duration = Some(duration.num_seconds() as i32);
            }
        }

        warn!("Marked session {} as failed: {}", session_id, error_message);
        Ok(())
    }

    /// 获取会话（优先从内存）
    pub async fn get_session(&self, session_id: &str) -> Option<Session> {
        // 先从内存查找
        let sessions = self.sessions.read().await;
        if let Some(session) = sessions.get(session_id) {
            return Some(session.clone());
        }
        drop(sessions);

        // 内存未找到，从数据库查询
        match sqlx::query_as::<_, SessionRecord>(
            r#"
            SELECT id, device_id, user_id, start_time, end_time,
                   duration, transcription, response, status
            FROM sessions
            WHERE id = $1
            "#
        )
        .bind(session_id)
        .fetch_optional(&self.db_pool)
        .await
        {
            Ok(Some(record)) => Some(record.into()),
            Ok(None) => None,
            Err(e) => {
                error!("Failed to fetch session {} from database: {}", session_id, e);
                None
            }
        }
    }

    /// 更新会话（保持向后兼容）
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

    /// 列出活跃会话（仅从内存）
    pub async fn list_active_sessions(&self) -> Vec<Session> {
        let sessions = self.sessions.read().await;
        sessions
            .values()
            .filter(|s| matches!(s.status, SessionStatus::Active))
            .cloned()
            .collect()
    }

    /// 清理内存中已完成的会话（保留在数据库）
    pub async fn cleanup_completed_sessions(&self) {
        let mut sessions = self.sessions.write().await;
        let before_count = sessions.len();

        sessions.retain(|_, session| {
            matches!(session.status, SessionStatus::Active)
        });

        let removed = before_count - sessions.len();
        if removed > 0 {
            info!("Cleaned up {} completed sessions from memory", removed);
        }
    }
}

// 数据库记录结构（用于查询）
#[derive(Debug, sqlx::FromRow)]
struct SessionRecord {
    id: String,
    device_id: String,
    user_id: Option<String>,
    start_time: chrono::DateTime<Utc>,
    end_time: Option<chrono::DateTime<Utc>>,
    duration: Option<i32>,
    transcription: Option<String>,
    response: Option<String>,
    status: String,
}

impl From<SessionRecord> for Session {
    fn from(record: SessionRecord) -> Self {
        Session {
            id: record.id,
            device_id: record.device_id,
            user_id: record.user_id, // 🔧 修复：直接使用 Option<String>，不需要 unwrap
            start_time: record.start_time,
            end_time: record.end_time,
            duration: record.duration,
            transcription: record.transcription,
            response: record.response,
            status: match record.status.as_str() {
                "active" => SessionStatus::Active,
                "completed" => SessionStatus::Completed,
                "failed" => SessionStatus::Failed,
                "timeout" => SessionStatus::Timeout,
                _ => SessionStatus::Failed,
            },
        }
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

    // 注意：测试需要真实数据库连接，这里保留原有测试结构
    // 实际测试应使用测试数据库或 mock

    #[tokio::test]
    #[ignore] // 需要数据库连接，默认跳过
    async fn test_session_lifecycle() {
        // 需要设置 DATABASE_URL 环境变量
        let database_url = std::env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set for integration tests");

        let pool = sqlx::PgPool::connect(&database_url)
            .await
            .expect("Failed to connect to database");

        let manager = SessionManager::new(pool);

        // 创建会话
        let session = manager.create_session("dev001", "user001")
            .await
            .expect("Failed to create session");

        assert_eq!(session.status, SessionStatus::Active);
        assert!(session.transcription.is_none());

        // 更新转录
        manager.update_transcription(&session.id, "Hello world".to_string())
            .await
            .expect("Failed to update transcription");

        // 完成会话
        manager.complete_session(
            &session.id,
            "Final transcription".to_string(),
            "Final response".to_string()
        )
        .await
        .expect("Failed to complete session");

        let completed = manager.get_session(&session.id).await;
        assert!(completed.is_some());
        assert_eq!(completed.unwrap().status, SessionStatus::Completed);

        // 清理测试数据
        sqlx::query!("DELETE FROM sessions WHERE id = $1", session.id)
            .execute(&manager.db_pool)
            .await
            .expect("Failed to cleanup test data");
    }
}
