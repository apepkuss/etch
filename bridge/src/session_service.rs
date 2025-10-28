// Bridge服务会话管理 - 存储层集成
use std::sync::Arc;
use anyhow::Result;
use sqlx::PgPool;
use echo_shared::{DatabaseError};
use echo_shared::database::SessionStatus;
use chrono::{DateTime, Utc};
use uuid::Uuid;

// 会话记录（对应数据库sessions表）
#[derive(Debug, Clone)]
pub struct SessionRecord {
    pub id: Uuid,
    pub device_id: Uuid,
    pub user_id: Option<Uuid>,
    pub status: SessionStatus,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub wake_reason: Option<String>,
    pub transcript: Option<String>,
    pub response: Option<String>,
    pub audio_url: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

// 会话服务
#[derive(Clone)]
pub struct SessionService {
    db: Arc<PgPool>,
}

impl SessionService {
    pub fn new(db: Arc<PgPool>) -> Self {
        Self { db }
    }

    /// 创建新会话
    pub async fn create_session(
        &self,
        device_id: &str,
        user_id: Option<&str>,
        wake_reason: Option<String>,
    ) -> Result<SessionRecord> {
        let session_id = Uuid::new_v4();
        let device_uuid = Uuid::parse_str(device_id)
            .map_err(|_| DatabaseError::InvalidInput("Invalid device ID".to_string()))?;

        let user_uuid = if let Some(uid) = user_id {
            Some(Uuid::parse_str(uid)
                .map_err(|_| DatabaseError::InvalidInput("Invalid user ID".to_string()))?)
        } else {
            None
        };

        let record = sqlx::query!(
            r#"
            INSERT INTO sessions (id, device_id, user_id, status, wake_reason)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, device_id, user_id, status as "status: SessionStatus",
                      started_at, ended_at, wake_reason, transcript, response, audio_url, metadata
            "#,
            session_id,
            device_uuid,
            user_uuid,
            "active" as SessionStatus,
            wake_reason
        )
        .fetch_one(self.db.as_ref())
        .await
        .map_err(DatabaseError::Connection)?;

        Ok(record)
    }

    /// 更新会话状态
    pub async fn update_session(
        &self,
        session_id: &str,
        status: SessionStatus,
        transcript: Option<String>,
        response: Option<String>,
        audio_url: Option<String>,
    ) -> Result<Option<SessionRecord>> {
        let session_uuid = Uuid::parse_str(session_id)
            .map_err(|_| DatabaseError::InvalidInput("Invalid session ID".to_string()))?;

        let record = sqlx::query_as!(
            SessionRecord,
            r#"
            UPDATE sessions
            SET status = $1,
                transcript = COALESCE($2, transcript),
                response = COALESCE($3, response),
                audio_url = COALESCE($4, audio_url),
                ended_at = CASE WHEN $5 = 'completed' THEN NOW() ELSE ended_at END
            WHERE id = $6
            RETURNING id, device_id, user_id, status as "status: SessionStatus",
                      started_at, ended_at, wake_reason, transcript, response, audio_url, metadata
            "#,
            status as SessionStatus,
            transcript,
            response,
            audio_url,
            session_uuid
        )
        .fetch_optional(self.db.as_ref())
        .await
        .map_err(|e| DatabaseError::Connection(e.to_string()))?;

        Ok(record)
    }

    /// 获取会话详情
    pub async fn get_session(&self, session_id: &str) -> Result<Option<SessionRecord>> {
        let session_uuid = Uuid::parse_str(session_id)
            .map_err(|_| DatabaseError::InvalidInput("Invalid session ID".to_string()))?;

        let record = sqlx::query_as!(
            SessionRecord,
            r#"
            SELECT id, device_id, user_id, status as "status: SessionStatus",
                   started_at, ended_at, wake_reason, transcript, response, audio_url, metadata
            FROM sessions
            WHERE id = $1
            "#,
            session_uuid
        )
        .fetch_optional(self.db.as_ref())
        .await
        .map_err(|e| DatabaseError::Connection(e.to_string()))?;

        Ok(record)
    }

    /// 获取设备会话列表
    pub async fn get_device_sessions(
        &self,
        device_id: &str,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<SessionRecord>> {
        let device_uuid = Uuid::parse_str(device_id)
            .map_err(|_| DatabaseError::InvalidInput("Invalid device ID".to_string()))?;

        let mut query = sqlx::query_as!(
            SessionRecord,
            r#"
            SELECT id, device_id, user_id, status as "status: SessionStatus",
                   started_at, ended_at, wake_reason, transcript, response, audio_url, metadata
            FROM sessions
            WHERE device_id = $1
            ORDER BY started_at DESC
            "#,
            device_uuid
        );

        if let Some(limit_val) = limit {
            query = query.limit(limit_val);
        }
        if let Some(offset_val) = offset {
            query = query.offset(offset_val);
        }

        let records = query
            .fetch_all(self.db.as_ref())
            .await
            .map_err(|e| DatabaseError::Connection(e.to_string()))?;

        Ok(records)
    }

    /// 获取用户会话列表
    pub async fn get_user_sessions(
        &self,
        user_id: &str,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<SessionRecord>> {
        let user_uuid = Uuid::parse_str(user_id)
            .map_err(|_| DatabaseError::InvalidInput("Invalid user ID".to_string()))?;

        let mut query = sqlx::query_as!(
            SessionRecord,
            r#"
            SELECT id, device_id, user_id, status as "status: SessionStatus",
                   started_at, ended_at, wake_reason, transcript, response, audio_url, metadata
            FROM sessions
            WHERE user_id = $1
            ORDER BY started_at DESC
            "#,
            user_uuid
        );

        if let Some(limit_val) = limit {
            query = query.limit(limit_val);
        }
        if let Some(offset_val) = offset {
            query = query.offset(offset_val);
        }

        let records = query
            .fetch_all(self.db.as_ref())
            .await
            .map_err(|e| DatabaseError::Connection(e.to_string()))?;

        Ok(records)
    }

    /// 获取活跃会话
    pub async fn get_active_sessions(&self) -> Result<Vec<SessionRecord>> {
        let records = sqlx::query_as!(
            SessionRecord,
            r#"
            SELECT id, device_id, user_id, status as "status: SessionStatus",
                   started_at, ended_at, wake_reason, transcript, response, audio_url, metadata
            FROM sessions
            WHERE status = 'active'
            ORDER BY started_at DESC
            "#
        )
        .fetch_all(self.db.as_ref())
        .await
        .map_err(|e| DatabaseError::Connection(e.to_string()))?;

        Ok(records)
    }

    /// 结束超时的会话
    pub async fn timeout_sessions(&self, timeout_minutes: i64) -> Result<u64> {
        let result = sqlx::query!(
            r#"
            UPDATE sessions
            SET status = 'timeout',
                ended_at = NOW()
            WHERE status = 'active'
              AND started_at < NOW() - INTERVAL '1 minute' * $1
            "#,
            timeout_minutes
        )
        .execute(self.db.as_ref())
        .await
        .map_err(|e| DatabaseError::Connection(e.to_string()))?;

        Ok(result.rows_affected())
    }

    /// 获取会话统计
    pub async fn get_session_stats(
        &self,
        device_id: Option<&str>,
        hours_back: Option<i32>,
    ) -> Result<SessionStats> {
        let mut query = sqlx::query!(
            r#"
            SELECT
                COUNT(*) as total_sessions,
                COUNT(CASE WHEN status = 'active' THEN 1 END) as active_sessions,
                COUNT(CASE WHEN status = 'completed' THEN 1 END) as completed_sessions,
                COUNT(CASE WHEN status = 'failed' THEN 1 END) as failed_sessions,
                COUNT(CASE WHEN status = 'timeout' THEN 1 END) as timeout_sessions,
                AVG(EXTRACT(EPOCH FROM (ended_at - started_at))/60) as avg_duration_minutes
            FROM sessions
            WHERE started_at >= NOW() - INTERVAL '1 hour' * $1
            "#,
            hours_back.unwrap_or(24)
        );

        if let Some(did) = device_id {
            let device_uuid = Uuid::parse_str(did)
                .map_err(|_| DatabaseError::InvalidInput("Invalid device ID".to_string()))?;
            query = query.bind(device_uuid);
        }

        let stats = query
            .fetch_one(self.db.as_ref())
            .await
            .map_err(|e| DatabaseError::Connection(e.to_string()))?;

        Ok(SessionStats {
            total_sessions: stats.total_sessions.unwrap_or(0) as i64,
            active_sessions: stats.active_sessions.unwrap_or(0) as i64,
            completed_sessions: stats.completed_sessions.unwrap_or(0) as i64,
            failed_sessions: stats.failed_sessions.unwrap_or(0) as i64,
            timeout_sessions: stats.timeout_sessions.unwrap_or(0) as i64,
            avg_duration_minutes: stats.avg_duration_minutes.map(|v| v as i64),
        })
    }
}

// 会话统计信息
#[derive(Debug, serde::Serialize)]
pub struct SessionStats {
    pub total_sessions: i64,
    pub active_sessions: i64,
    pub completed_sessions: i64,
    pub failed_sessions: i64,
    pub timeout_sessions: i64,
    pub avg_duration_minutes: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_crud() {
        // 这里需要模拟数据库连接，实际测试需要test database
        // 暂时跳过实际测试
    }
}