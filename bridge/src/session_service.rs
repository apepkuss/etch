// Bridge服务会话管理 - 存储层集成
use std::sync::Arc;
use anyhow::Result;
use sqlx::{PgPool, Row, FromRow};
use echo_shared::{DatabaseError};
use echo_shared::database::SessionStatus;
use chrono::{DateTime, Utc};

// 会话记录（对应数据库sessions表）
// 注意：数据库使用 VARCHAR(255) 存储 ID，支持自定义格式如 "session_xxx" 和 "ECHO_ES20500101002_xxx"
#[derive(Debug, Clone, FromRow)]
pub struct SessionRecord {
    pub id: String,           // 会话ID，支持任意字符串格式
    pub device_id: String,    // 设备ID，支持任意字符串格式
    pub user_id: Option<String>, // 用户ID，支持任意字符串格式
    pub status: String,       // 会话状态：active, completed, failed, timeout
    #[sqlx(rename = "start_time")]
    pub started_at: DateTime<Utc>,  // 数据库字段名为 start_time
    #[sqlx(rename = "end_time")]
    pub ended_at: Option<DateTime<Utc>>, // 数据库字段名为 end_time
    // 注意：数据库 schema 中没有 wake_reason 字段
    #[sqlx(rename = "transcription")]
    pub transcript: Option<String>, // 数据库字段名为 transcription
    pub response: Option<String>,
    #[sqlx(rename = "audio_file_path")]
    pub audio_url: Option<String>,  // 数据库字段名为 audio_file_path
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
        _wake_reason: Option<String>, // 保留参数兼容性，但不使用
    ) -> Result<SessionRecord> {
        // 直接使用字符串 ID，不再解析为 UUID
        // 数据库 schema 已经使用 VARCHAR(255)，支持任意格式的 ID
        let clean_session_id = session_id.to_string();
        let clean_device_id = device_id.to_string();
        let clean_user_id = user_id.map(|s| s.to_string());

        let status_str = match SessionStatus::Active {
            SessionStatus::Active => "active",
            SessionStatus::Completed => "completed",
            SessionStatus::Failed => "failed",
            SessionStatus::Timeout => "timeout",
        };

        let record = sqlx::query_as::<_, SessionRecord>(
            r#"
            INSERT INTO sessions (id, device_id, user_id, status)
            VALUES ($1, $2, $3, $4)
            RETURNING id, device_id, user_id, status,
                      start_time, end_time, transcription, response, audio_file_path, metadata
            "#
        )
        .bind(clean_session_id)
        .bind(clean_device_id)
        .bind(clean_user_id)
        .bind(status_str)
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
        // 直接使用字符串 ID
        let clean_session_id = session_id.to_string();

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
                transcription = COALESCE($2, transcription),
                response = COALESCE($3, response),
                audio_file_path = COALESCE($4, audio_file_path),
                end_time = CASE WHEN $1 = 'completed' THEN NOW() ELSE end_time END,
                duration = CASE WHEN $1 = 'completed' THEN EXTRACT(EPOCH FROM (NOW() - start_time))::INTEGER ELSE duration END
            WHERE id = $5
            RETURNING id, device_id, user_id, status,
                      start_time, end_time, transcription, response, audio_file_path, metadata
            "#
        )
        .bind(status_str)
        .bind(transcript)
        .bind(response)
        .bind(audio_url)
        .bind(clean_session_id)
        .fetch_optional(self.db.as_ref())
        .await
        .map_err(DatabaseError::Connection)?;

        Ok(record)
    }

    /// 获取会话详情
    pub async fn get_session(&self, session_id: &str) -> Result<Option<SessionRecord>> {
        // 直接使用字符串 ID
        let clean_session_id = session_id.to_string();

        let record = sqlx::query_as::<_, SessionRecord>(
            r#"
            SELECT id, device_id, user_id, status,
                   start_time, end_time, transcription, response, audio_file_path, metadata
            FROM sessions
            WHERE id = $1
            "#
        )
        .bind(clean_session_id)
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
        // 直接使用字符串 ID
        let clean_device_id = device_id.to_string();

        let sql = r#"
            SELECT id, device_id, user_id, status,
                   start_time, end_time, transcription, response, audio_file_path, metadata
            FROM sessions
            WHERE device_id = $1
            ORDER BY start_time DESC
            LIMIT $2 OFFSET $3
        "#;

        let records = sqlx::query_as::<_, SessionRecord>(sql)
            .bind(clean_device_id)
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
        // 直接使用字符串 ID
        let clean_user_id = user_id.to_string();

        let sql = r#"
            SELECT id, device_id, user_id, status,
                   start_time, end_time, transcription, response, audio_file_path, metadata
            FROM sessions
            WHERE user_id = $1
            ORDER BY start_time DESC
            LIMIT $2 OFFSET $3
        "#;

        let records = sqlx::query_as::<_, SessionRecord>(sql)
            .bind(clean_user_id)
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
                   start_time, end_time, transcription, response, audio_file_path, metadata
            FROM sessions
            WHERE status = 'active'
            ORDER BY start_time DESC
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
                end_time = NOW()
            WHERE status = 'active'
              AND start_time < NOW() - INTERVAL '1 minute' * $1
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

        let (sql, device_id_value) = if let Some(did) = device_id {
            // 直接使用字符串 ID
            let clean_device_id = did.to_string();
            (
                r#"
                SELECT
                    COUNT(*) as "total!",
                    COUNT(CASE WHEN status = 'active' THEN 1 END) as "active!",
                    COUNT(CASE WHEN status = 'completed' THEN 1 END) as "completed!",
                    COUNT(CASE WHEN status = 'failed' THEN 1 END) as "failed!",
                    COUNT(CASE WHEN status = 'timeout' THEN 1 END) as "timeout!",
                    AVG(EXTRACT(EPOCH FROM (end_time - start_time))/60) as avg_duration
                FROM sessions
                WHERE start_time >= NOW() - INTERVAL '1 hour' * $1
                  AND device_id = $2
                "#,
                Some(clean_device_id)
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
                    AVG(EXTRACT(EPOCH FROM (end_time - start_time))/60) as avg_duration
                FROM sessions
                WHERE start_time >= NOW() - INTERVAL '1 hour' * $1
                "#,
                None
            )
        };

        let row = if let Some(did) = device_id_value {
            sqlx::query(sql)
                .bind(hours)
                .bind(did)
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
    ///
    /// 注意：此方法当前未被使用，保留以备将来需要
    pub async fn ensure_device_exists(
        &self,
        device_id: &str,
        device_name: Option<&str>,
    ) -> Result<String, DatabaseError> {
        // 直接使用字符串 ID
        let clean_device_id = device_id.to_string();

        // 检查设备是否已存在
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM devices WHERE id = $1)"
        )
        .bind(&clean_device_id)
        .fetch_one(&*self.db)
        .await
        .map_err(|e| DatabaseError::InvalidInput(format!("Failed to check device existence: {}", e)))?;

        if exists {
            return Ok(clean_device_id);
        }

        // 设备不存在，创建新记录
        let name = device_name.unwrap_or("WebUI 设备");

        sqlx::query(
            "INSERT INTO devices (id, name, device_type, status, created_at, updated_at)
             VALUES ($1, $2, 'web_browser', 'online', NOW(), NOW())
             ON CONFLICT (id) DO NOTHING"
        )
        .bind(&clean_device_id)
        .bind(name)
        .execute(&*self.db)
        .await
        .map_err(|e| DatabaseError::InvalidInput(format!("Failed to create device: {}", e)))?;

        tracing::info!(
            "Created new WebUI device: id={}, name={}",
            clean_device_id,
            name
        );

        Ok(clean_device_id)
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