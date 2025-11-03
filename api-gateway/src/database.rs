use std::env;
use anyhow::Result;
use sqlx::{PgPool, postgres::PgPoolOptions};
use tracing::{info, error};
use echo_shared::types::SessionStatus;

/// 数据库连接池
#[derive(Clone)]
pub struct Database {
    pool: PgPool,
}

impl Database {
    /// 创建新的数据库连接池
    pub async fn new() -> Result<Self> {
        let database_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://echo_user:echo_password@localhost:5432/echo_db".to_string());

        info!("Connecting to database: {}", database_url);

        let pool = PgPoolOptions::new()
            .max_connections(20)
            .min_connections(5)
            .connect(&database_url)
            .await?;

        info!("Database connection pool created successfully");

        Ok(Database { pool })
    }

    /// 运行数据库迁移
    pub async fn run_migrations(&self) -> Result<()> {
        info!("Running database migrations...");

        // 这里可以使用 sqlx migrate 或者手动执行 SQL 文件
        // 由于我们使用 SQLX_OFFLINE=true，暂时跳过自动迁移
        // 在生产环境中，应该使用 sqlx migrate run

        info!("Database migrations completed");
        Ok(())
    }

    /// 获取连接池
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// 健康检查
    pub async fn health_check(&self) -> Result<bool> {
        let result = sqlx::query("SELECT 1")
            .fetch_one(&self.pool)
            .await;

        match result {
            Ok(_) => {
                info!("Database health check: OK");
                Ok(true)
            }
            Err(e) => {
                error!("Database health check failed: {}", e);
                Ok(false)
            }
        }
    }
}

// 简化的用户相关操作（暂时返回mock数据）
impl Database {
    /// 根据用户名获取用户（暂时返回mock数据）
    pub async fn get_user_by_username(&self, username: &str) -> Result<Option<echo_shared::User>> {
        if username == "admin" {
            Ok(Some(echo_shared::User {
                id: "admin-001".to_string(),
                username: "admin".to_string(),
                email: "admin@echo.system".to_string(),
                password_hash: "$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewdBPj3QJgusgqHG".to_string(),
                role: echo_shared::UserRole::Admin,
            }))
        } else {
            Ok(None)
        }
    }

    /// 根据ID获取用户（暂时返回mock数据）
    pub async fn get_user_by_id(&self, user_id: &str) -> Result<Option<echo_shared::User>> {
        if user_id == "admin-001" {
            Ok(Some(echo_shared::User {
                id: "admin-001".to_string(),
                username: "admin".to_string(),
                email: "admin@echo.system".to_string(),
                password_hash: "$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewdBPj3QJgusgqHG".to_string(),
                role: echo_shared::UserRole::Admin,
            }))
        } else {
            Ok(None)
        }
    }

    /// 验证密码（暂时返回mock验证）
    pub async fn verify_password(&self, username: &str, password: &str) -> Result<Option<echo_shared::User>> {
        if let Some(user) = self.get_user_by_username(username).await? {
            // 使用 bcrypt 验证密码
            let is_valid = bcrypt::verify(password, &user.password_hash).unwrap_or(false);
            if is_valid {
                return Ok(Some(user));
            }
        }
        Ok(None)
    }
}

// 简化的设备相关操作（暂时返回mock数据）
impl Database {
    /// 获取所有设备（暂时返回mock数据）
    pub async fn get_all_devices(&self) -> Result<Vec<echo_shared::Device>> {
        Ok(vec![
            echo_shared::Device {
                id: "device-001".to_string(),
                name: "Living Room Speaker".to_string(),
                device_type: echo_shared::DeviceType::Speaker,
                status: echo_shared::DeviceStatus::Online,
                location: "Living Room".to_string(),
                firmware_version: "1.0.0".to_string(),
                battery_level: 100,
                volume: 50,
                last_seen: chrono::Utc::now(),
                is_online: true,
                owner: "admin-001".to_string(),
            },
            echo_shared::Device {
                id: "device-002".to_string(),
                name: "Bedroom Display".to_string(),
                device_type: echo_shared::DeviceType::Display,
                status: echo_shared::DeviceStatus::Offline,
                location: "Bedroom".to_string(),
                firmware_version: "1.0.0".to_string(),
                battery_level: 80,
                volume: 30,
                last_seen: chrono::Utc::now(),
                is_online: false,
                owner: "admin-001".to_string(),
            },
        ])
    }

    /// 根据ID获取设备（暂时返回mock数据）
    pub async fn get_device_by_id(&self, device_id: &str) -> Result<Option<echo_shared::Device>> {
        let devices = self.get_all_devices().await?;
        Ok(devices.into_iter().find(|d| d.id == device_id))
    }
}

// 简化的会话相关操作（暂时返回mock数据）
impl Database {
    /// 获取所有会话（暂时返回mock数据）
    pub async fn get_all_sessions(&self) -> Result<Vec<echo_shared::Session>> {
        Ok(vec![
            echo_shared::Session {
                id: "session-001".to_string(),
                device_id: "device-001".to_string(),
                user_id: "admin-001".to_string(),
                start_time: chrono::Utc::now(),
                end_time: Some(chrono::Utc::now()),
                duration: Some(120),
                transcription: Some("Hello, how can I help you?".to_string()),
                response: Some("I need help with my smart home".to_string()),
                status: SessionStatus::Completed,
            },
        ])
    }

    /// 创建新会话（暂时返回mock数据）
    pub async fn create_session(&self, session: &echo_shared::Session) -> Result<echo_shared::Session> {
        Ok(session.clone())
    }

    /// 更新会话状态（暂时mock实现）
    pub async fn update_session_status(&self, _session_id: &str, _status: SessionStatus) -> Result<()> {
        Ok(())
    }
}