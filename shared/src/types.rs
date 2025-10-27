use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

// 设备相关类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Device {
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
    pub owner: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DeviceType {
    Speaker,
    Display,
    Hub,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DeviceStatus {
    Online,
    Offline,
    Maintenance,
    Error,
}

// 设备配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceConfig {
    pub volume: Option<i32>,
    pub location: Option<String>,
    pub battery_level: Option<i32>,
}

// 用户相关类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub role: UserRole,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UserRole {
    Admin,
    User,
    Guest,
}

// 会话相关类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub device_id: String,
    pub user_id: String,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub duration: Option<i32>,
    pub transcription: Option<String>,
    pub response: Option<String>,
    pub status: SessionStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SessionStatus {
    Active,
    Completed,
    Failed,
    Timeout,
}

// API 请求/响应类型
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub message: String,
    pub timestamp: DateTime<Utc>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: "Success".to_string(),
            timestamp: Utc::now(),
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            message,
            timestamp: Utc::now(),
        }
    }
}

// JWT Claims
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // 用户ID
    pub username: String,
    pub role: UserRole,
    pub exp: usize, // 过期时间
    pub iat: usize, // 签发时间
}

// WebSocket 消息类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WebSocketMessage {
    DeviceStatusUpdate {
        device_id: String,
        status: DeviceStatus,
        timestamp: DateTime<Utc>,
    },
    SessionProgress {
        session_id: String,
        device_id: String,
        stage: SessionStage,
        progress: f32,
        message: String,
    },
    SystemNotification {
        level: NotificationLevel,
        title: String,
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionStage {
    Wakeup,
    Listening,
    Processing,
    Responding,
    Completed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationLevel {
    Info,
    Warning,
    Error,
    Success,
}

// MQTT 消息类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttMessage {
    pub device_id: String,
    pub message_type: String,
    pub payload: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}

// 音频相关类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioChunk {
    pub device_id: String,
    pub sequence_number: u32,
    pub data: Vec<u8>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionResult {
    pub text: String,
    pub confidence: f32,
    pub language: String,
    pub timestamp: DateTime<Utc>,
}

// 错误类型
#[derive(Debug, thiserror::Error)]
pub enum EchoError {
    #[error("Database error: {0}")]
    Database(String),

    #[error("Redis error: {0}")]
    Redis(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("JWT error: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),

    #[error("Password hashing error: {0}")]
    Bcrypt(#[from] bcrypt::BcryptError),

    #[error("Authentication error: {0}")]
    Authentication(String),

    #[error("Authorization error: {0}")]
    Authorization(String),

    #[error("Device not found: {0}")]
    DeviceNotFound(String),

    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Internal server error: {0}")]
    Internal(#[from] anyhow::Error),
}

// 分页相关类型
#[derive(Debug, Serialize, Deserialize)]
pub struct PaginationParams {
    pub page: u32,
    pub page_size: u32,
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self {
            page: 1,
            page_size: 20,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub items: Vec<T>,
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
    pub total_pages: u32,
}

impl<T> PaginatedResponse<T> {
    pub fn new(items: Vec<T>, total: u64, params: PaginationParams) -> Self {
        let total_pages = ((total as f64) / (params.page_size as f64)).ceil() as u32;
        Self {
            items,
            total,
            page: params.page,
            page_size: params.page_size,
            total_pages,
        }
    }
}

// 配置相关类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub mqtt: MqttConfig,
    pub jwt: JwtConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub workers: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    pub url: String,
    pub max_connections: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttConfig {
    pub broker: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtConfig {
    pub secret: String,
    pub expiration_hours: u64,
}