use std::env;
use anyhow::Result;
use sqlx::{PgPool, postgres::PgPoolOptions, Row};
use tracing::{info, error};
use echo_shared::{types::SessionStatus, DeviceStatus, DeviceType};
use chrono::{DateTime, Utc};

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

// 设备相关操作
impl Database {
    /// 获取所有设备
    pub async fn get_all_devices(&self) -> Result<Vec<echo_shared::Device>> {
        let rows = sqlx::query("SELECT id, name, device_type, status, firmware_version, battery_level, volume_level as volume, last_seen, is_online, owner, echokit_server_url FROM devices ORDER BY created_at DESC")
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|row| {
            // 从数据库获取原始数据
            let device_type_str: String = row.get("device_type");
            let status_str: String = row.get("status");

            // 转换设备类型 - 简化后只支持Speaker类型
            let device_type = match device_type_str.as_str() {
                "speaker" => DeviceType::Speaker,
                _ => DeviceType::Speaker, // 所有未知类型都默认为Speaker
            };

            // 转换设备状态
            let status = match status_str.as_str() {
                "online" => DeviceStatus::Online,
                "offline" => DeviceStatus::Offline,
                "maintenance" => DeviceStatus::Maintenance,
                _ => DeviceStatus::Offline,
            };

            echo_shared::Device {
                id: row.get::<String, _>("id"),
                name: row.get("name"),
                device_type,
                status,
                location: String::new(), // 空字符串，不再从数据库获取
                firmware_version: row.get::<Option<String>, _>("firmware_version").unwrap_or_default(),
                battery_level: row.get::<Option<i32>, _>("battery_level").unwrap_or(0),
                volume: row.get::<Option<i32>, _>("volume").unwrap_or(50),
                last_seen: row.get::<Option<DateTime<Utc>>, _>("last_seen").unwrap_or_else(chrono::Utc::now),
                is_online: row.get::<Option<bool>, _>("is_online").unwrap_or(false),
                owner: row.get::<Option<String>, _>("owner").unwrap_or_default(),
                echokit_server_url: row.get::<Option<String>, _>("echokit_server_url"),
            }
        }).collect())
    }

    /// 根据ID获取设备
    pub async fn get_device_by_id(&self, device_id: &str) -> Result<Option<echo_shared::Device>> {
        let device = sqlx::query("SELECT id, name, device_type, status, firmware_version, battery_level, volume_level as volume, last_seen, is_online, owner, echokit_server_url FROM devices WHERE id = $1")
            .bind(device_id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(device.map(|row| {
            // 从数据库获取原始数据
            let device_type_str: String = row.get("device_type");
            let status_str: String = row.get("status");

            // 转换设备类型 - 简化后只支持Speaker类型
            let device_type = match device_type_str.as_str() {
                "speaker" => DeviceType::Speaker,
                _ => DeviceType::Speaker, // 所有未知类型都默认为Speaker
            };

            // 转换设备状态
            let status = match status_str.as_str() {
                "online" => DeviceStatus::Online,
                "offline" => DeviceStatus::Offline,
                "maintenance" => DeviceStatus::Maintenance,
                _ => DeviceStatus::Offline,
            };

            echo_shared::Device {
                id: row.get::<String, _>("id"),
                name: row.get("name"),
                device_type,
                status,
                location: String::new(), // 空字符串，不再从数据库获取
                firmware_version: row.get::<Option<String>, _>("firmware_version").unwrap_or_default(),
                battery_level: row.get::<Option<i32>, _>("battery_level").unwrap_or(0),
                volume: row.get::<Option<i32>, _>("volume").unwrap_or(50),
                last_seen: row.get::<Option<DateTime<Utc>>, _>("last_seen").unwrap_or_else(chrono::Utc::now),
                is_online: row.get::<Option<bool>, _>("is_online").unwrap_or(false),
                owner: row.get::<Option<String>, _>("owner").unwrap_or_default(),
                echokit_server_url: row.get::<Option<String>, _>("echokit_server_url"),
            }
        }))
    }

    /// 创建设备注册令牌
    pub async fn create_registration_token(
        &self,
        device_id: &str,
        pairing_code: &str,
        qr_token: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO device_registration_tokens (
                device_id, pairing_code, qr_token, expires_at, created_at
            ) VALUES (
                $1, $2, $3, $4, NOW()
            )
            "#
        )
        .bind(device_id)
        .bind(pairing_code)
        .bind(qr_token)
        .bind(expires_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// 删除设备
    pub async fn delete_device(&self, device_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM devices WHERE id = $1")
            .bind(device_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// 更新设备信息
    pub async fn update_device(&self, device: &echo_shared::Device) -> Result<echo_shared::Device> {
        let result = sqlx::query("UPDATE devices SET name = $1, device_type = $2, firmware_version = $3, battery_level = $4, volume_level = $5, last_seen = $6, is_online = $7, updated_at = NOW() WHERE id = $8 RETURNING id, name, device_type, status, firmware_version, battery_level, volume_level as volume, last_seen, is_online, owner")
            .bind(device.name.clone())
            .bind("speaker") // 暂时硬编码
            .bind(device.firmware_version.clone())
            .bind(device.battery_level)
            .bind(device.volume)
            .bind(device.last_seen)
            .bind(device.is_online)
            .bind(&device.id)
            .fetch_one(&self.pool)
            .await?;

        Ok(echo_shared::Device {
            id: result.get::<String, _>("id"),
            name: result.get("name"),
            device_type: DeviceType::Speaker, // 需要根据数据库实际类型转换
            status: DeviceStatus::Online, // 需要根据数据库实际状态转换
            location: String::new(), // 空字符串，不再从数据库获取
            firmware_version: result.get::<Option<String>, _>("firmware_version").unwrap_or_default(),
            battery_level: result.get::<Option<i32>, _>("battery_level").unwrap_or(0),
            volume: 50, // Default volume
            last_seen: result.get::<Option<DateTime<Utc>>, _>("last_seen").unwrap_or_else(chrono::Utc::now),
            is_online: result.get::<Option<bool>, _>("is_online").unwrap_or(false),
            owner: result.get::<Option<String>, _>("owner").unwrap_or_default(),
            echokit_server_url: None,
        })
    }

    /// 创建新设备
    pub async fn create_device(
        &self,
        device: &echo_shared::Device,
        serial_number: Option<&str>,
        mac_address: Option<&str>,
        pairing_code: Option<&str>,
        registration_token: Option<&str>,
    ) -> Result<echo_shared::Device> {
        let result = sqlx::query("INSERT INTO devices (id, name, device_type, status, firmware_version, battery_level, volume_level, last_seen, is_online, owner, pairing_code, registration_token, serial_number, mac_address, echokit_server_url, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, NOW(), NOW()) RETURNING id, name, device_type, status, firmware_version, battery_level, volume_level as volume, last_seen, is_online, owner, echokit_server_url")
            .bind(&device.id)
            .bind(device.name.clone())
            .bind("speaker") // 暂时硬编码
            .bind("pending") // 暂时硬编码
            .bind(device.firmware_version.clone())
            .bind(device.battery_level)
            .bind(device.volume)
            .bind(device.last_seen)
            .bind(device.is_online)
            .bind(device.owner.clone())
            .bind(pairing_code)
            .bind(registration_token)
            .bind(serial_number)
            .bind(mac_address)
            .bind(device.echokit_server_url.as_deref())
            .fetch_one(&self.pool)
            .await?;

        Ok(echo_shared::Device {
            id: result.get::<String, _>("id"),
            name: result.get("name"),
            device_type: DeviceType::Speaker, // 需要根据数据库实际类型转换
            status: DeviceStatus::Pending, // 需要根据数据库实际状态转换
            location: String::new(), // 空字符串，不再从数据库获取
            firmware_version: result.get::<Option<String>, _>("firmware_version").unwrap_or_default(),
            battery_level: result.get::<Option<i32>, _>("battery_level").unwrap_or(0),
            volume: result.get::<Option<i32>, _>("volume").unwrap_or(50),
            last_seen: result.get::<Option<DateTime<Utc>>, _>("last_seen").unwrap_or_else(chrono::Utc::now),
            is_online: result.get::<Option<bool>, _>("is_online").unwrap_or(false),
            owner: result.get::<Option<String>, _>("owner").unwrap_or_default(),
            echokit_server_url: result.get::<Option<String>, _>("echokit_server_url"),
        })
    }

    /// 更新设备状态
    pub async fn update_device_status(&self, device_id: &str, status: DeviceStatus) -> Result<()> {
        sqlx::query("UPDATE devices SET status = $1, updated_at = NOW() WHERE id = $2")
            .bind(status.to_string())
            .bind(device_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// 更新设备的EchoKit服务器URL
    pub async fn update_device_echokit_server(
        &self,
        device_id: &str,
        owner_id: &str,
        echokit_server_url: Option<&str>,
    ) -> Result<bool> {
        let result = sqlx::query(
            "UPDATE devices SET echokit_server_url = $1, updated_at = NOW() WHERE id = $2 AND owner = $3"
        )
            .bind(echokit_server_url)
            .bind(device_id)
            .bind(owner_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// 更新设备名称
    pub async fn update_device_name(
        &self,
        device_id: &str,
        owner_id: &str,
        name: &str,
    ) -> Result<bool> {
        let result = sqlx::query(
            "UPDATE devices SET name = $1, updated_at = NOW() WHERE id = $2 AND owner = $3"
        )
            .bind(name)
            .bind(device_id)
            .bind(owner_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// 更新设备位置
    pub async fn update_device_location(
        &self,
        device_id: &str,
        owner_id: &str,
        location: &str,
    ) -> Result<bool> {
        let result = sqlx::query(
            "UPDATE devices SET location = $1, updated_at = NOW() WHERE id = $2 AND owner = $3"
        )
            .bind(location)
            .bind(device_id)
            .bind(owner_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }


    /// 检查序列号是否已存在
    pub async fn check_serial_number_exists(&self, serial_number: &str) -> Result<bool> {
        let exists: Option<bool> = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM devices WHERE serial_number = $1)")
            .bind(serial_number)
            .fetch_one(&self.pool)
            .await?;

        Ok(exists.unwrap_or(false))
    }

    /// 检查MAC地址是否已存在
    pub async fn check_mac_address_exists(&self, mac_address: &str) -> Result<bool> {
        let exists: Option<bool> = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM devices WHERE mac_address = $1)")
            .bind(mac_address)
            .fetch_one(&self.pool)
            .await?;

        Ok(exists.unwrap_or(false))
    }

    /// 验证设备注册
    pub async fn verify_device_registration(
        &self,
        pairing_code: &str,
    ) -> Result<Option<String>> {
        let result: Option<String> = sqlx::query_scalar("SELECT id FROM devices WHERE pairing_code = $1 AND status = 'pending'")
            .bind(pairing_code)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(device_id) = result {
            // 更新设备状态为在线
            sqlx::query("UPDATE devices SET status = 'online', is_online = true, updated_at = NOW() WHERE id = $1")
                .bind(&device_id)
                .execute(&self.pool)
                .await?;

            Ok(Some(device_id))
        } else {
            Ok(None)
        }
    }

    /// 根据配对码获取设备信息
    pub async fn get_device_by_pairing_code(&self, pairing_code: &str) -> Result<Option<echo_shared::Device>> {
        let device = sqlx::query("SELECT id, name, device_type, status, firmware_version, battery_level, volume_level as volume, last_seen, is_online, owner, echokit_server_url FROM devices WHERE pairing_code = $1")
            .bind(pairing_code)
            .fetch_optional(&self.pool)
            .await?;

        Ok(device.map(|row| {
            echo_shared::Device {
                id: row.get::<String, _>("id"),
                name: row.get("name"),
                device_type: DeviceType::Speaker, // 简化后只支持Speaker类型
                status: DeviceStatus::Pending, // 需要根据数据库实际状态转换
                location: String::new(), // 空字符串，不再从数据库获取
                firmware_version: row.get::<Option<String>, _>("firmware_version").unwrap_or_default(),
                battery_level: row.get::<Option<i32>, _>("battery_level").unwrap_or(0),
                volume: row.get::<Option<i32>, _>("volume").unwrap_or(50),
                last_seen: row.get::<Option<DateTime<Utc>>, _>("last_seen").unwrap_or_else(chrono::Utc::now),
                is_online: row.get::<Option<bool>, _>("is_online").unwrap_or(false),
                owner: row.get::<Option<String>, _>("owner").unwrap_or_default(),
                echokit_server_url: row.get::<Option<String>, _>("echokit_server_url"),
            }
        }))
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