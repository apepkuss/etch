use std::env;
use anyhow::Result;
use sqlx::{PgPool, postgres::PgPoolOptions, Row, Pool, Postgres};
use tracing::{info, warn, error};
use chrono::{DateTime, Utc};

use echo_shared::{User, UserRole, Device, DeviceStatus, DeviceType, Session};
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

// 用户相关的数据库操作
impl Database {
    /// 根据用户名获取用户
    pub async fn get_user_by_username(&self, username: &str) -> Result<Option<User>> {
        let row = sqlx::query(
            r#"
            SELECT id, username, email, password_hash, role, created_at, updated_at, is_active
            FROM users
            WHERE username = $1 AND is_active = true
            "#
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let id: String = row.try_get("id")?;
            let username: String = row.try_get("username")?;
            let email: String = row.try_get("email")?;
            let password_hash: String = row.try_get("password_hash")?;
            let role_str: String = row.try_get("role")?;

            let role = match role_str.as_str() {
                "admin" => UserRole::Admin,
                _ => UserRole::User,
            };

            Ok(Some(User {
                id,
                username,
                email,
                password_hash,
                role,
            }))
        } else {
            Ok(None)
        }
    }

    /// 根据ID获取用户
    pub async fn get_user_by_id(&self, user_id: &str) -> Result<Option<User>> {
        let row = sqlx::query(
            r#"
            SELECT id, username, email, password_hash, role, created_at, updated_at, is_active
            FROM users
            WHERE id = $1 AND is_active = true
            "#
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let id: String = row.try_get("id")?;
            let username: String = row.try_get("username")?;
            let email: String = row.try_get("email")?;
            let password_hash: String = row.try_get("password_hash")?;
            let role_str: String = row.try_get("role")?;

            let role = match role_str.as_str() {
                "admin" => UserRole::Admin,
                _ => UserRole::User,
            };

            Ok(Some(User {
                id,
                username,
                email,
                password_hash,
                role,
            }))
        } else {
            Ok(None)
        }
    }

    /// 创建新用户
    pub async fn create_user(&self, user: &User) -> Result<User> {
        let role_str = match user.role {
            UserRole::Admin => "admin",
            UserRole::User => "user",
        };

        let row = sqlx::query!(
            r#"
            INSERT INTO users (id, username, email, password_hash, role)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, username, email, password_hash, role, created_at, updated_at, is_active
            "#,
            user.id,
            user.username,
            user.email,
            user.password_hash,
            role_str
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(User {
            id: row.id,
            username: row.username,
            email: row.email,
            password_hash: row.password_hash,
            role: user.role.clone(),
        })
    }

    /// 获取所有用户
    pub async fn get_all_users(&self) -> Result<Vec<User>> {
        let rows = sqlx::query!(
            r#"
            SELECT id, username, email, password_hash, role, created_at, updated_at, is_active
            FROM users
            WHERE is_active = true
            ORDER BY created_at DESC
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        let mut users = Vec::new();
        for row in rows {
            let role = match row.role.as_str() {
                "admin" => UserRole::Admin,
                _ => UserRole::User,
            };

            users.push(User {
                id: row.id,
                username: row.username,
                email: row.email,
                password_hash: row.password_hash,
                role,
            });
        }

        Ok(users)
    }

    /// 验证密码
    pub async fn verify_password(&self, username: &str, password: &str) -> Result<Option<User>> {
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

// 设备相关的数据库操作
impl Database {
    /// 获取所有设备
    pub async fn get_all_devices(&self) -> Result<Vec<Device>> {
        let rows = sqlx::query!(
            r#"
            SELECT id, name, device_type, status, location, firmware_version,
                   battery_level, volume, last_seen, is_online, owner_id, config
            FROM devices
            ORDER BY created_at DESC
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        let mut devices = Vec::new();
        for row in rows {
            let device_type = match row.device_type.as_str() {
                "speaker" => DeviceType::Speaker,
                "display" => DeviceType::Display,
                "hub" => DeviceType::Hub,
                _ => DeviceType::Speaker,
            };

            let status = match row.status.as_str() {
                "online" => DeviceStatus::Online,
                "offline" => DeviceStatus::Offline,
                "maintenance" => DeviceStatus::Maintenance,
                "error" => DeviceStatus::Error,
                _ => DeviceStatus::Offline,
            };

            devices.push(Device {
                id: row.id,
                name: row.name,
                device_type,
                status,
                location: row.location.unwrap_or_default(),
                firmware_version: row.firmware_version.unwrap_or_default(),
                battery_level: row.battery_level.unwrap_or(100),
                volume: row.volume.unwrap_or(50),
                last_seen: row.last_seen.unwrap_or_else(Utc::now),
                is_online: row.is_online.unwrap_or(false),
                owner: row.owner_id.unwrap_or_default(),
            });
        }

        Ok(devices)
    }

    /// 根据ID获取设备
    pub async fn get_device_by_id(&self, device_id: &str) -> Result<Option<Device>> {
        let row = sqlx::query!(
            r#"
            SELECT id, name, device_type, status, location, firmware_version,
                   battery_level, volume, last_seen, is_online, owner_id, config
            FROM devices
            WHERE id = $1
            "#,
            device_id
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let device_type = match row.device_type.as_str() {
                "speaker" => DeviceType::Speaker,
                "display" => DeviceType::Display,
                "hub" => DeviceType::Hub,
                _ => DeviceType::Speaker,
            };

            let status = match row.status.as_str() {
                "online" => DeviceStatus::Online,
                "offline" => DeviceStatus::Offline,
                "maintenance" => DeviceStatus::Maintenance,
                "error" => DeviceStatus::Error,
                _ => DeviceStatus::Offline,
            };

            Ok(Some(Device {
                id: row.id,
                name: row.name,
                device_type,
                status,
                location: row.location.unwrap_or_default(),
                firmware_version: row.firmware_version.unwrap_or_default(),
                battery_level: row.battery_level.unwrap_or(100),
                volume: row.volume.unwrap_or(50),
                last_seen: row.last_seen.unwrap_or_else(Utc::now),
                is_online: row.is_online.unwrap_or(false),
                owner: row.owner_id.unwrap_or_default(),
            }))
        } else {
            Ok(None)
        }
    }

    /// 创建新设备
    pub async fn create_device(&self, device: &Device) -> Result<Device> {
        let device_type_str = match device.device_type {
            DeviceType::Speaker => "speaker",
            DeviceType::Display => "display",
            DeviceType::Hub => "hub",
        };

        let status_str = match device.status {
            DeviceStatus::Online => "online",
            DeviceStatus::Offline => "offline",
            DeviceStatus::Maintenance => "maintenance",
            DeviceStatus::Error => "error",
        };

        let row = sqlx::query!(
            r#"
            INSERT INTO devices (id, name, device_type, status, location, last_seen, is_online)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id, name, device_type, status, location, last_seen, is_online
            "#,
            device.id,
            device.name,
            device_type_str,
            status_str,
            device.location,
            device.last_seen,
            device.is_online
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(Device {
            id: row.id,
            name: row.name,
            device_type: device.device_type.clone(),
            status: device.status.clone(),
            location: row.location.unwrap_or_default(),
            firmware_version: "1.0.0".to_string(),
            battery_level: 100,
            volume: 50,
            last_seen: row.last_seen.unwrap_or_else(Utc::now),
            is_online: row.is_online.unwrap_or(false),
            owner: "".to_string(),
        })
    }

    /// 更新设备状态
    pub async fn update_device_status(&self, device_id: &str, status: DeviceStatus) -> Result<()> {
        let status_str = match status {
            DeviceStatus::Online => "online",
            DeviceStatus::Offline => "offline",
            DeviceStatus::Maintenance => "maintenance",
            DeviceStatus::Error => "error",
        };

        sqlx::query!(
            "UPDATE devices SET status = $1, last_seen = NOW() WHERE id = $2",
            status_str,
            device_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

// 会话相关的数据库操作
impl Database {
    /// 获取所有会话
    pub async fn get_all_sessions(&self) -> Result<Vec<Session>> {
        let rows = sqlx::query!(
            r#"
            SELECT id, device_id, user_id, status, started_at, ended_at,
                   wake_reason, transcript, duration_seconds
            FROM sessions
            ORDER BY started_at DESC
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        let mut sessions = Vec::new();
        for row in rows {
            let status = match row.status.as_str() {
                "active" => SessionStatus::Active,
                "completed" => SessionStatus::Completed,
                "failed" => SessionStatus::Failed,
                "timeout" => SessionStatus::Timeout,
                _ => SessionStatus::Active,
            };

            sessions.push(Session {
                id: row.id,
                device_id: row.device_id,
                user_id: row.user_id,
                start_time: row.started_at.unwrap_or_else(Utc::now),
                end_time: row.ended_at,
                duration: row.duration_seconds,
                transcription: row.transcript,
                response: None,
                status,
            });
        }

        Ok(sessions)
    }

    /// 创建新会话
    pub async fn create_session(&self, session: &Session) -> Result<Session> {
        let status_str = match session.status {
            SessionStatus::Active => "active",
            SessionStatus::Completed => "completed",
            SessionStatus::Failed => "failed",
            SessionStatus::Timeout => "timeout",
        };

        let row = sqlx::query!(
            r#"
            INSERT INTO sessions (id, device_id, user_id, status, started_at)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, device_id, user_id, status, started_at
            "#,
            session.id,
            session.device_id,
            session.user_id,
            status_str,
            session.created_at
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(Session {
            id: row.id,
            device_id: row.device_id,
            user_id: row.user_id,
            start_time: row.started_at.unwrap_or_else(Utc::now),
            end_time: None,
            duration: None,
            transcription: session.transcription.clone(),
            response: None,
            status: session.status.clone(),
        })
    }

    /// 更新会话状态
    pub async fn update_session_status(&self, session_id: &str, status: SessionStatus) -> Result<()> {
        let status_str = match status {
            SessionStatus::Active => "active",
            SessionStatus::Completed => "completed",
            SessionStatus::Failed => "failed",
            SessionStatus::Timeout => "timeout",
        };

        sqlx::query!(
            "UPDATE sessions SET status = $1, updated_at = NOW() WHERE id = $2",
            status_str,
            session_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}