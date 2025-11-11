// Bridge服务会话管理 - 存储层集成
use std::sync::Arc;
use anyhow::Result;
use sqlx::{PgPool, Row, FromRow};
use echo_shared::{DatabaseError};
use echo_shared::database::SessionStatus;
use chrono::{DateTime, Utc};
use uuid::Uuid;

// 会话记录（对应数据库sessions表）
#[derive(Debug, Clone, FromRow)]
pub struct SessionRecord {
    pub id: Uuid,
    pub device_id: Uuid,
    pub user_id: Option<Uuid>,
    pub status: String, // 使用 String 而不是 SessionStatus，避免编译时类型检查
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
        session_id: &str,
        device_id: &str,
        user_id: Option<&str>,
        wake_reason: Option<String>,
    ) -> Result<SessionRecord> {
        let session_uuid = Uuid::parse_str(session_id)
            .map_err(|_| DatabaseError::InvalidInput("Invalid session ID".to_string()))?;
        let device_uuid = Uuid::parse_str(device_id)
            .map_err(|_| DatabaseError::InvalidInput("Invalid device ID".to_string()))?;

        let user_uuid = if let Some(uid) = user_id {
            Some(Uuid::parse_str(uid)
                .map_err(|_| DatabaseError::InvalidInput("Invalid user ID".to_string()))?)
        } else {
            None
        };

        let status_str = match SessionStatus::Active {
            SessionStatus::Active => "active",
            SessionStatus::Completed => "completed",
            SessionStatus::Failed => "failed",
            SessionStatus::Timeout => "timeout",
        };

        let record = sqlx::query_as::<_, SessionRecord>(
            r#"
            INSERT INTO sessions (id, device_id, user_id, status, wake_reason)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, device_id, user_id, status,
                      started_at, ended_at, wake_reason, transcript, response, audio_url, metadata
            "#
        )
        .bind(session_uuid)
        .bind(device_uuid)
        .bind(user_uuid)
        .bind(status_str)
        .bind(wake_reason)
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

        let status_str = match status {
            SessionStatus::Active => "active",
            SessionStatus::Completed => "completed",
            SessionStatus::Failed => "failed",
            SessionStatus::Timeout => "timeout",
        };

        let record = sqlx::query_as::<_, SessionRecord>(
            r#"
            UPDATE sessions
            SET status = $1,
                transcript = COALESCE($2, transcript),
                response = COALESCE($3, response),
                audio_url = COALESCE($4, audio_url),
                ended_at = CASE WHEN $1 = 'completed' THEN NOW() ELSE ended_at END
            WHERE id = $5
            RETURNING id, device_id, user_id, status,
                      started_at, ended_at, wake_reason, transcript, response, audio_url, metadata
            "#
        )
        .bind(status_str)
        .bind(transcript)
        .bind(response)
        .bind(audio_url)
        .bind(session_uuid)
        .fetch_optional(self.db.as_ref())
        .await
        .map_err(DatabaseError::Connection)?;

        Ok(record)
    }

    /// 获取会话详情
    pub async fn get_session(&self, session_id: &str) -> Result<Option<SessionRecord>> {
        let session_uuid = Uuid::parse_str(session_id)
            .map_err(|_| DatabaseError::InvalidInput("Invalid session ID".to_string()))?;

        let record = sqlx::query_as::<_, SessionRecord>(
            r#"
            SELECT id, device_id, user_id, status,
                   started_at, ended_at, wake_reason, transcript, response, audio_url, metadata
            FROM sessions
            WHERE id = $1
            "#
        )
        .bind(session_uuid)
        .fetch_optional(self.db.as_ref())
        .await
        .map_err(DatabaseError::Connection)?;

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

        let sql = r#"
            SELECT id, device_id, user_id, status,
                   started_at, ended_at, wake_reason, transcript, response, audio_url, metadata
            FROM sessions
            WHERE device_id = $1
            ORDER BY started_at DESC
            LIMIT $2 OFFSET $3
        "#;

        let records = sqlx::query_as::<_, SessionRecord>(sql)
            .bind(device_uuid)
            .bind(limit.unwrap_or(100))
            .bind(offset.unwrap_or(0))
            .fetch_all(self.db.as_ref())
            .await
            .map_err(DatabaseError::Connection)?;

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

        let sql = r#"
            SELECT id, device_id, user_id, status,
                   started_at, ended_at, wake_reason, transcript, response, audio_url, metadata
            FROM sessions
            WHERE user_id = $1
            ORDER BY started_at DESC
            LIMIT $2 OFFSET $3
        "#;

        let records = sqlx::query_as::<_, SessionRecord>(sql)
            .bind(user_uuid)
            .bind(limit.unwrap_or(100))
            .bind(offset.unwrap_or(0))
            .fetch_all(self.db.as_ref())
            .await
            .map_err(DatabaseError::Connection)?;

        Ok(records)
    }

    /// 获取活跃会话
    pub async fn get_active_sessions(&self) -> Result<Vec<SessionRecord>> {
        let records = sqlx::query_as::<_, SessionRecord>(
            r#"
            SELECT id, device_id, user_id, status,
                   started_at, ended_at, wake_reason, transcript, response, audio_url, metadata
            FROM sessions
            WHERE status = 'active'
            ORDER BY started_at DESC
            "#
        )
        .fetch_all(self.db.as_ref())
        .await
        .map_err(DatabaseError::Connection)?;

        Ok(records)
    }

    /// 结束超时的会话
    pub async fn timeout_sessions(&self, timeout_minutes: i64) -> Result<u64> {
        let result = sqlx::query(
            r#"
            UPDATE sessions
            SET status = 'timeout',
                ended_at = NOW()
            WHERE status = 'active'
              AND started_at < NOW() - INTERVAL '1 minute' * $1
            "#
        )
        .bind(timeout_minutes)
        .execute(self.db.as_ref())
        .await
        .map_err(DatabaseError::Connection)?;

        Ok(result.rows_affected())
    }

    /// 获取会话统计
    pub async fn get_session_stats(
        &self,
        device_id: Option<&str>,
        hours_back: Option<i32>,
    ) -> Result<SessionStats> {
        let hours = hours_back.unwrap_or(24);

        let (sql, device_uuid) = if let Some(did) = device_id {
            let uuid = Uuid::parse_str(did)
                .map_err(|_| DatabaseError::InvalidInput("Invalid device ID".to_string()))?;
            (
                r#"
                SELECT
                    COUNT(*) as "total!",
                    COUNT(CASE WHEN status = 'active' THEN 1 END) as "active!",
                    COUNT(CASE WHEN status = 'completed' THEN 1 END) as "completed!",
                    COUNT(CASE WHEN status = 'failed' THEN 1 END) as "failed!",
                    COUNT(CASE WHEN status = 'timeout' THEN 1 END) as "timeout!",
                    AVG(EXTRACT(EPOCH FROM (ended_at - started_at))/60) as avg_duration
                FROM sessions
                WHERE started_at >= NOW() - INTERVAL '1 hour' * $1
                  AND device_id = $2
                "#,
                Some(uuid)
            )
        } else {
            (
                r#"
                SELECT
                    COUNT(*) as "total!",
                    COUNT(CASE WHEN status = 'active' THEN 1 END) as "active!",
                    COUNT(CASE WHEN status = 'completed' THEN 1 END) as "completed!",
                    COUNT(CASE WHEN status = 'failed' THEN 1 END) as "failed!",
                    COUNT(CASE WHEN status = 'timeout' THEN 1 END) as "timeout!",
                    AVG(EXTRACT(EPOCH FROM (ended_at - started_at))/60) as avg_duration
                FROM sessions
                WHERE started_at >= NOW() - INTERVAL '1 hour' * $1
                "#,
                None
            )
        };

        let row = if let Some(uuid) = device_uuid {
            sqlx::query(sql)
                .bind(hours)
                .bind(uuid)
                .fetch_one(self.db.as_ref())
                .await
        } else {
            sqlx::query(sql)
                .bind(hours)
                .fetch_one(self.db.as_ref())
                .await
        }.map_err(DatabaseError::Connection)?;

        Ok(SessionStats {
            total_sessions: row.try_get("total").unwrap_or(0),
            active_sessions: row.try_get("active").unwrap_or(0),
            completed_sessions: row.try_get("completed").unwrap_or(0),
            failed_sessions: row.try_get("failed").unwrap_or(0),
            timeout_sessions: row.try_get("timeout").unwrap_or(0),
            avg_duration_minutes: row.try_get::<Option<f64>, _>("avg_duration").ok().flatten().map(|v| v as i64),
        })
    }

    /// 确保设备存在（如果不存在则创建）
    ///
    /// 对于 WebUI 连接，使用浏览器指纹 (visitor_id) 作为设备 ID
    /// 如果设备不存在，则自动创建一个 WebUI 设备记录
    pub async fn ensure_device_exists(
        &self,
        device_id: &str,
        device_name: Option<&str>,
    ) -> Result<Uuid, DatabaseError> {
        // 解析 device_id 为 UUID，如果失败则创建新的 UUID
        let device_uuid = if let Ok(uuid) = Uuid::parse_str(device_id) {
            uuid
        } else {
            // visitor_id 不是标准 UUID 格式，使用确定性哈希转换为 UUID
            // 这样同一个 visitor_id 总是得到相同的 UUID
            let hash = md5::compute(device_id);
            let hash_bytes = hash.0;
            Uuid::from_slice(&hash_bytes).map_err(|e| {
                DatabaseError::InvalidInput(format!("Failed to create UUID from visitor_id: {}", e))
            })?
        };

        // 检查设备是否已存在
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM devices WHERE id = $1)"
        )
        .bind(&device_uuid)
        .fetch_one(&*self.db)
        .await
        .map_err(|e| DatabaseError::InvalidInput(format!("Failed to check device existence: {}", e)))?;

        if exists {
            return Ok(device_uuid);
        }

        // 设备不存在，创建新记录
        let name = device_name.unwrap_or("WebUI 设备");

        sqlx::query(
            "INSERT INTO devices (id, name, device_type, status, created_at, updated_at)
             VALUES ($1, $2, 'web_browser', 'online', NOW(), NOW())
             ON CONFLICT (id) DO NOTHING"
        )
        .bind(&device_uuid)
        .bind(name)
        .execute(&*self.db)
        .await
        .map_err(|e| DatabaseError::InvalidInput(format!("Failed to create device: {}", e)))?;

        tracing::info!(
            "Created new WebUI device: id={}, name={}, visitor_id={}",
            device_uuid,
            name,
            device_id
        );

        Ok(device_uuid)
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