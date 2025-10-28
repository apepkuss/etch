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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UserRole {
    Admin,
    User,
    Viewer,
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
    // 原有消息类型
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

    // EchoKit 集成消息类型
    EchoKitSessionStart {
        session_id: String,
        device_id: String,
        config: EchoKitConfig,
    },
    EchoKitSessionEnd {
        session_id: String,
        device_id: String,
        reason: String,
    },
    EchoKitAudioData {
        device_id: String,
        session_id: String,
        audio_data: Vec<u8>,
        format: AudioFormat,
    },
    EchoKitTranscription {
        session_id: String,
        device_id: String,
        text: String,
        confidence: f32,
        is_final: bool,
        timestamp: DateTime<Utc>,
    },
    EchoKitResponse {
        session_id: String,
        device_id: String,
        text: String,
        audio_data: Option<Vec<u8>>,
        is_complete: bool,
        timestamp: DateTime<Utc>,
    },
    EchoKitError {
        session_id: String,
        device_id: String,
        error_code: String,
        error_message: String,
        timestamp: DateTime<Utc>,
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

// EchoKit 集成相关类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EchoKitConfig {
    pub vad_enabled: bool,
    pub vad_threshold: f32,
    pub vad_silence_duration: f32,
    pub asr_model: String,
    pub asr_language: String,
    pub llm_model: String,
    pub llm_provider: String,
    pub tts_voice: String,
    pub tts_provider: String,
    pub stream_response: bool,
    pub max_audio_length: f32,
    pub session_timeout: f32,
}

impl Default for EchoKitConfig {
    fn default() -> Self {
        Self {
            vad_enabled: true,
            vad_threshold: 0.5,
            vad_silence_duration: 1.0,
            asr_model: "whisper".to_string(),
            asr_language: "zh".to_string(),
            llm_model: "gpt-3.5-turbo".to_string(),
            llm_provider: "openai".to_string(),
            tts_voice: "zh-CN-female-1".to_string(),
            tts_provider: "azure".to_string(),
            stream_response: true,
            max_audio_length: 30.0,
            session_timeout: 60.0,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum AudioFormat {
    PCM16,
    WAV,
    Opus,
    MP3,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EchoKitSession {
    pub id: String,
    pub device_id: String,
    pub user_id: String,
    pub config: EchoKitConfig,
    pub status: EchoKitSessionStatus,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub current_stage: SessionStage,
    pub progress: f32,
    pub transcription: Option<String>,
    pub response: Option<String>,
    pub audio_buffer: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EchoKitSessionStatus {
    Initializing,
    Active,
    Processing,
    Responding,
    Completed,
    Failed,
    Timeout,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EchoKitDevice {
    pub device_id: String,
    pub websocket_url: String,
    pub is_connected: bool,
    pub last_heartbeat: DateTime<Utc>,
    pub current_session_id: Option<String>,
    pub audio_format: AudioFormat,
    pub sample_rate: u32,
    pub channels: u8,
}

// EchoKit 服务状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EchoKitServiceStatus {
    pub is_connected: bool,
    pub websocket_url: String,
    pub last_heartbeat: DateTime<Utc>,
    pub active_sessions: u32,
    pub max_sessions: u32,
    pub supported_formats: Vec<AudioFormat>,
    pub service_version: String,
}

// EchoKit 统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EchoKitStats {
    pub total_sessions: u64,
    pub successful_sessions: u64,
    pub failed_sessions: u64,
    pub average_response_time: f32,
    pub total_audio_processed: f64, // in seconds
    pub current_active_sessions: u32,
    pub uptime: f64, // in seconds
}

// EchoKit 错误类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EchoKitErrorInfo {
    pub code: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
    pub timestamp: DateTime<Utc>,
    pub session_id: Option<String>,
    pub device_id: Option<String>,
}

// EchoKit 客户端消息 (发送到 EchoKit Server)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum EchoKitClientMessage {
    StartSession {
        session_id: String,
        device_id: String,
        config: EchoKitConfig,
    },
    EndSession {
        session_id: String,
        device_id: String,
        reason: String,
    },
    AudioData {
        session_id: String,
        device_id: String,
        audio_data: Vec<u8>,
        format: AudioFormat,
        is_final: bool,
    },
    Ping,
}

// EchoKit 服务端消息 (从 EchoKit Server 接收) - 兼容OpenAI realtime API
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum EchoKitServerMessage {
    // OpenAI realtime API 格式消息
    #[serde(rename = "session.created")]
    SessionCreated {
        event_id: String,
        session: OpenAISession,
    },

    #[serde(rename = "conversation.created")]
    ConversationCreated {
        event_id: String,
        conversation: OpenAIConversation,
    },

    #[serde(rename = "response.text")]
    ResponseText {
        event_id: String,
        session_id: String,
        text: String,
    },

    #[serde(rename = "response.audio")]
    ResponseAudio {
        event_id: String,
        session_id: String,
        audio: String, // Base64 encoded audio
    },

    #[serde(rename = "error")]
    OpenAIError {
        event_id: String,
        error: OpenAIError,
    },

    // 原有格式消息（向后兼容）
    SessionStarted {
        session_id: String,
        device_id: String,
        timestamp: DateTime<Utc>,
    },
    SessionEnded {
        session_id: String,
        device_id: String,
        reason: String,
        timestamp: DateTime<Utc>,
    },
    Transcription {
        session_id: String,
        device_id: String,
        text: String,
        confidence: f32,
        is_final: bool,
        timestamp: DateTime<Utc>,
    },
    Response {
        session_id: String,
        device_id: String,
        text: String,
        audio_data: Option<Vec<u8>>,
        is_complete: bool,
        timestamp: DateTime<Utc>,
    },
    Error {
        session_id: String,
        device_id: String,
        error: EchoKitErrorInfo,
    },
    Pong,
    ServiceStatus {
        status: EchoKitServiceStatus,
    },
}

// 便利函数
pub fn now_utc() -> DateTime<Utc> {
    Utc::now()
}

pub fn generate_session_id() -> String {
    use uuid::Uuid;
    Uuid::new_v4().to_string()
}

pub fn generate_device_id() -> String {
    use uuid::Uuid;
    format!("dev_{}", Uuid::new_v4().to_string()[..8].to_lowercase())
}

// ============================================================================
// 简化版 OpenAI realtime API 协议类型定义
// ============================================================================

// OpenAI Session 信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAISession {
    pub id: String,
    pub object: String, // "realtime.session"
    pub model: String,
    pub modalities: Vec<String>, // ["text", "audio"]
    pub instructions: Option<String>,
    pub voice: Option<String>,
    pub input_audio_format: String, // "pcm16"
    pub output_audio_format: String, // "pcm16"
    pub temperature: Option<f32>,
}

// OpenAI Conversation 信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIConversation {
    pub id: String,
    pub object: String, // "realtime.conversation"
}

// OpenAI Error 信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIError {
    pub code: Option<String>,
    pub message: String,
    pub param: Option<String>,
    pub type_: String, // "invalid_request_error", etc.
}

// OpenAI 客户端事件类型 (发送到服务器)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum OpenAIClientEvent {
    #[serde(rename = "session.update")]
    SessionUpdate {
        event_id: Option<String>,
        session: OpenAISessionConfig,
    },

    #[serde(rename = "input_audio_buffer.append")]
    InputAudioBufferAppend {
        event_id: Option<String>,
        audio: String, // Base64 encoded audio
    },

    #[serde(rename = "response.create")]
    ResponseCreate {
        event_id: Option<String>,
        response: Option<OpenAIResponseConfig>,
    },
}

// OpenAI Session 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAISessionConfig {
    pub instructions: Option<String>,
    pub voice: Option<String>,
    pub temperature: Option<f32>,
}

// OpenAI Response 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIResponseConfig {
    pub modalities: Option<Vec<String>>,
    pub instructions: Option<String>,
    pub voice: Option<String>,
}