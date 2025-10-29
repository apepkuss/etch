// 数据库模型和SQL查询定义
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use crate::{Device, DeviceStatus, DeviceType, User, UserRole};

// 数据库模型（对应PostgreSQL表结构）

// 用户表模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRecord {
    pub id: String,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub role: UserRole,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_active: bool,
}

// 设备表模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceRecord {
    pub id: String,
    pub name: String,
    pub device_type: DeviceType,
    pub status: DeviceStatus,
    pub location: String,
    pub firmware_version: String,
    pub battery_level: i32,
    pub volume: i32,
    pub last_seen: DateTime<Utc>,
    pub is_online: bool,
    pub owner_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub config: serde_json::Value, // JSON配置存储
}

// 会话表模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRecord {
    pub id: String,
    pub device_id: String,
    pub user_id: Option<String>,
    pub status: SessionStatus,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub wake_reason: Option<String>,
    pub transcript: Option<String>, // ASR转录文本
    pub response: Option<String>,  // LLM响应
    pub audio_url: Option<String>, // 音频文件URL
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionStatus {
    Active,
    Completed,
    Failed,
    Timeout,
}

// 用户设备关联表模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserDeviceRecord {
    pub user_id: String,
    pub device_id: String,
    pub permission: DevicePermission,
    pub granted_at: DateTime<Utc>,
    pub granted_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DevicePermission {
    Owner,
    Admin,
    User,
    Viewer,
}

// 创建新用户的请求
#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub email: String,
    pub password: String,
    pub role: UserRole,
}

// 创建新设备的请求
#[derive(Debug, Deserialize)]
pub struct CreateDeviceRequest {
    pub name: String,
    pub device_type: DeviceType,
    pub location: Option<String>,
    pub firmware_version: Option<String>,
}

// 更新设备的请求
#[derive(Debug, Deserialize)]
pub struct UpdateDeviceRequest {
    pub name: Option<String>,
    pub location: Option<String>,
    pub volume: Option<i32>,
    pub config: Option<serde_json::Value>,
}

// 设备查询过滤器
#[derive(Debug, Deserialize)]
pub struct DeviceFilter {
    pub owner_id: Option<String>,
    pub device_type: Option<DeviceType>,
    pub status: Option<DeviceStatus>,
    pub is_online: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// SQL查询常量
pub mod queries {
    pub const CREATE_USERS_TABLE: &str = r#"
        CREATE TABLE IF NOT EXISTS users (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            username VARCHAR(50) UNIQUE NOT NULL,
            email VARCHAR(255) UNIQUE NOT NULL,
            password_hash VARCHAR(255) NOT NULL,
            role VARCHAR(20) NOT NULL DEFAULT 'user',
            created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
            updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
            is_active BOOLEAN DEFAULT TRUE
        );
    "#;

    pub const CREATE_DEVICES_TABLE: &str = r#"
        CREATE TABLE IF NOT EXISTS devices (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            name VARCHAR(100) NOT NULL,
            device_type VARCHAR(20) NOT NULL,
            status VARCHAR(20) NOT NULL DEFAULT 'offline',
            location VARCHAR(100),
            firmware_version VARCHAR(50),
            battery_level INTEGER DEFAULT 0,
            volume INTEGER DEFAULT 50,
            last_seen TIMESTAMP WITH TIME ZONE,
            is_online BOOLEAN DEFAULT FALSE,
            owner_id UUID NOT NULL REFERENCES users(id),
            created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
            updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
            config JSONB DEFAULT '{}'
        );
    "#;

    pub const CREATE_SESSIONS_TABLE: &str = r#"
        CREATE TABLE IF NOT EXISTS sessions (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            device_id UUID NOT NULL REFERENCES devices(id),
            user_id UUID REFERENCES users(id),
            status VARCHAR(20) NOT NULL DEFAULT 'active',
            started_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
            ended_at TIMESTAMP WITH TIME ZONE,
            wake_reason VARCHAR(50),
            transcript TEXT,
            response TEXT,
            audio_url VARCHAR(500),
            metadata JSONB DEFAULT '{}'
        );
    "#;

    pub const CREATE_USER_DEVICES_TABLE: &str = r#"
        CREATE TABLE IF NOT EXISTS user_devices (
            user_id UUID NOT NULL REFERENCES users(id),
            device_id UUID NOT NULL REFERENCES devices(id),
            permission VARCHAR(20) NOT NULL DEFAULT 'owner',
            granted_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
            granted_by UUID NOT NULL REFERENCES users(id),
            PRIMARY KEY (user_id, device_id)
        );
    "#;

    // 索引创建
    pub const CREATE_INDEXES: &str = r#"
        CREATE INDEX IF NOT EXISTS idx_devices_owner_id ON devices(owner_id);
        CREATE INDEX IF NOT EXISTS idx_devices_status ON devices(status);
        CREATE INDEX IF NOT EXISTS idx_devices_last_seen ON devices(last_seen);
        CREATE INDEX IF NOT EXISTS idx_sessions_device_id ON sessions(device_id);
        CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON sessions(user_id);
        CREATE INDEX IF NOT EXISTS idx_sessions_started_at ON sessions(started_at);
        CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);
        CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
    "#;

    // 用户查询
    pub const GET_USER_BY_ID: &str = r#"
        SELECT id, username, email, password_hash, role, created_at, updated_at, is_active
        FROM users
        WHERE id = $1 AND is_active = TRUE
    "#;

    pub const GET_USER_BY_USERNAME: &str = r#"
        SELECT id, username, email, password_hash, role, created_at, updated_at, is_active
        FROM users
        WHERE username = $1 AND is_active = TRUE
    "#;

    pub const GET_USER_BY_EMAIL: &str = r#"
        SELECT id, username, email, password_hash, role, created_at, updated_at, is_active
        FROM users
        WHERE email = $1 AND is_active = TRUE
    "#;

    pub const CREATE_USER: &str = r#"
        INSERT INTO users (username, email, password_hash, role)
        VALUES ($1, $2, $3, $4)
        RETURNING id, username, email, password_hash, role, created_at, updated_at, is_active
    "#;

    // 设备查询
    pub const GET_DEVICES_BY_OWNER: &str = r#"
        SELECT id, name, device_type, status, location, firmware_version,
               battery_level, volume, last_seen, is_online, owner_id,
               created_at, updated_at, config
        FROM devices
        WHERE owner_id = $1
        ORDER BY last_seen DESC
        LIMIT $2 OFFSET $3
    "#;

    pub const GET_DEVICE_BY_ID: &str = r#"
        SELECT id, name, device_type, status, location, firmware_version,
               battery_level, volume, last_seen, is_online, owner_id,
               created_at, updated_at, config
        FROM devices
        WHERE id = $1
    "#;

    pub const CREATE_DEVICE: &str = r#"
        INSERT INTO devices (name, device_type, location, firmware_version, owner_id)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, name, device_type, status, location, firmware_version,
                  battery_level, volume, last_seen, is_online, owner_id,
                  created_at, updated_at, config
    "#;

    pub const UPDATE_DEVICE: &str = r#"
        UPDATE devices
        SET name = COALESCE($1, name),
            location = COALESCE($2, location),
            volume = COALESCE($3, volume),
            config = COALESCE($4, config),
            updated_at = NOW()
        WHERE id = $5
        RETURNING id, name, device_type, status, location, firmware_version,
                  battery_level, volume, last_seen, is_online, owner_id,
                  created_at, updated_at, config
    "#;

    pub const UPDATE_DEVICE_STATUS: &str = r#"
        UPDATE devices
        SET status = $1,
            battery_level = COALESCE($2, battery_level),
            volume = COALESCE($3, volume),
            last_seen = COALESCE($4, NOW()),
            is_online = COALESCE($5, is_online),
            updated_at = NOW()
        WHERE id = $6
    "#;

    // 会话查询
    pub const CREATE_SESSION: &str = r#"
        INSERT INTO sessions (device_id, user_id, status, wake_reason)
        VALUES ($1, $2, $3, $4)
        RETURNING id, device_id, user_id, status, started_at, ended_at,
                  wake_reason, transcript, response, audio_url, metadata
    "#;

    pub const UPDATE_SESSION: &str = r#"
        UPDATE sessions
        SET status = COALESCE($1, status),
            transcript = COALESCE($2, transcript),
            response = COALESCE($3, response),
            audio_url = COALESCE($4, audio_url),
            ended_at = COALESCE($5, ended_at),
            metadata = COALESCE($6, metadata)
        WHERE id = $7
        RETURNING id, device_id, user_id, status, started_at, ended_at,
                  wake_reason, transcript, response, audio_url, metadata
    "#;

    pub const GET_DEVICE_SESSIONS: &str = r#"
        SELECT id, device_id, user_id, status, started_at, ended_at,
               wake_reason, transcript, response, audio_url, metadata
        FROM sessions
        WHERE device_id = $1
        ORDER BY started_at DESC
        LIMIT $2 OFFSET $3
    "#;
}

// 数据库错误类型
#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    #[error("Database connection error: {0}")]
    Connection(#[from] sqlx::Error),

    #[error("User not found: {0}")]
    UserNotFound(String),

    #[error("Device not found: {0}")]
    DeviceNotFound(String),

    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Duplicate record: {0}")]
    DuplicateRecord(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),
}