// Redis缓存层定义
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use crate::{Device, DeviceStatus};

// Redis键名常量
pub mod keys {
    pub const DEVICE_LIST_PREFIX: &str = "devices:list:";
    pub const DEVICE_STATUS_PREFIX: &str = "device:status:";
    pub const DEVICE_CONFIG_PREFIX: &str = "device:config:";
    pub const USER_SESSION_PREFIX: &str = "user:session:";
    pub const USER_TOKEN_PREFIX: &str = "user:token:";
    pub const MQTT_CONNECTION_PREFIX: &str = "mqtt:conn:";
}

// 缓存项过期时间（秒）
pub mod ttl {
    pub const DEVICE_LIST: u64 = 60;        // 设备列表缓存60秒
    pub const DEVICE_STATUS: u64 = 300;     // 设备状态缓存5分钟
    pub const DEVICE_CONFIG: u64 = 600;     // 设备配置缓存10分钟
    pub const USER_SESSION: u64 = 3600;     // 用户会话1小时
    pub const USER_TOKEN: u64 = 86400;      // 用户Token 24小时
    pub const MQTT_CONNECTION: u64 = 120;   // MQTT连接状态2分钟
}

// 缓存的数据结构

// 设备状态缓存
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceStatusCache {
    pub device_id: String,
    pub status: DeviceStatus,
    pub battery_level: Option<i32>,
    pub volume: Option<i32>,
    pub location: Option<String>,
    pub last_seen: DateTime<Utc>,
    pub is_online: bool,
}

// 设备配置缓存
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceConfigCache {
    pub device_id: String,
    pub volume: Option<i32>,
    pub location: Option<String>,
    pub language: Option<String>,
    pub timezone: Option<String>,
    pub wake_word_enabled: Option<bool>,
    pub auto_reply_enabled: Option<bool>,
    pub custom_settings: Option<serde_json::Value>,
    pub updated_at: DateTime<Utc>,
}

// 用户会话缓存
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSessionCache {
    pub user_id: String,
    pub username: String,
    pub role: String,
    pub permissions: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

// MQTT连接状态缓存
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttConnectionCache {
    pub client_id: String,
    pub service_name: String,
    pub status: String,
    pub connected_at: DateTime<Utc>,
    pub last_heartbeat: DateTime<Utc>,
}

// 缓存操作接口
#[async_trait::async_trait]
pub trait CacheOperations: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;

    // 基本操作
    async fn get<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<Option<T>, Self::Error>;
    async fn set<T: Serialize + std::marker::Sync>(&self, key: &str, value: &T, ttl_seconds: u64) -> Result<(), Self::Error>;
    async fn delete(&self, key: &str) -> Result<bool, Self::Error>;
    async fn exists(&self, key: &str) -> Result<bool, Self::Error>;
    async fn expire(&self, key: &str, ttl_seconds: u64) -> Result<bool, Self::Error>;

    // 列表操作
    async fn list_push<T: Serialize + std::marker::Sync>(&self, key: &str, value: &T) -> Result<(), Self::Error>;
    async fn list_pop<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<Option<T>, Self::Error>;
    async fn list_len(&self, key: &str) -> Result<usize, Self::Error>;

    // 哈希操作
    async fn hash_set<T: Serialize + std::marker::Sync>(&self, key: &str, field: &str, value: &T) -> Result<(), Self::Error>;
    async fn hash_get<T: for<'de> Deserialize<'de>>(&self, key: &str, field: &str) -> Result<Option<T>, Self::Error>;
    async fn hash_delete(&self, key: &str, field: &str) -> Result<bool, Self::Error>;
    async fn hash_exists(&self, key: &str, field: &str) -> Result<bool, Self::Error>;

    // 模式匹配删除
    async fn delete_pattern(&self, pattern: &str) -> Result<u64, Self::Error>;
}

// Redis缓存操作实现
pub struct RedisCache {
    client: redis::Client,
}

impl RedisCache {
    pub fn new(connection_string: &str) -> Result<Self, redis::RedisError> {
        let client = redis::Client::open(connection_string)?;
        Ok(Self { client })
    }

    async fn get_connection(&self) -> Result<redis::aio::MultiplexedConnection, redis::RedisError> {
        self.client.get_multiplexed_async_connection().await
    }

    // 生成设备列表缓存键
    pub fn device_list_key(owner_id: &str) -> String {
        format!("{}{}", keys::DEVICE_LIST_PREFIX, owner_id)
    }

    // 生成设备状态缓存键
    pub fn device_status_key(device_id: &str) -> String {
        format!("{}{}", keys::DEVICE_STATUS_PREFIX, device_id)
    }

    // 生成设备配置缓存键
    pub fn device_config_key(device_id: &str) -> String {
        format!("{}{}", keys::DEVICE_CONFIG_PREFIX, device_id)
    }

    // 生成用户会话缓存键
    pub fn user_session_key(user_id: &str) -> String {
        format!("{}{}", keys::USER_SESSION_PREFIX, user_id)
    }

    // 生成用户Token缓存键
    pub fn user_token_key(token: &str) -> String {
        format!("{}{}", keys::USER_TOKEN_PREFIX, token)
    }

    // 生成MQTT连接缓存键
    pub fn mqtt_connection_key(client_id: &str) -> String {
        format!("{}{}", keys::MQTT_CONNECTION_PREFIX, client_id)
    }
}

#[async_trait::async_trait]
impl CacheOperations for RedisCache {
    type Error = redis::RedisError;

    async fn get<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<Option<T>, Self::Error> {
        let mut conn = self.get_connection().await?;
        let value: Option<String> = redis::cmd("GET").arg(key).query_async(&mut conn).await?;

        match value {
            Some(json_str) => {
                let item: T = serde_json::from_str(&json_str)
                    .map_err(|e| redis::RedisError::from((redis::ErrorKind::TypeError, "JSON deserialization failed", e.to_string())))?;
                Ok(Some(item))
            }
            None => Ok(None),
        }
    }

    async fn set<T: Serialize + std::marker::Sync>(&self, key: &str, value: &T, ttl_seconds: u64) -> Result<(), Self::Error> {
        let mut conn = self.get_connection().await?;
        let json_str = serde_json::to_string(value)
            .map_err(|e| redis::RedisError::from((redis::ErrorKind::TypeError, "JSON serialization failed", e.to_string())))?;

        redis::cmd("SETEX")
            .arg(key)
            .arg(ttl_seconds)
            .arg(json_str)
            .query_async(&mut conn)
            .await
    }

    async fn delete(&self, key: &str) -> Result<bool, Self::Error> {
        let mut conn = self.get_connection().await?;
        let count: i32 = redis::cmd("DEL").arg(key).query_async(&mut conn).await?;
        Ok(count > 0)
    }

    async fn exists(&self, key: &str) -> Result<bool, Self::Error> {
        let mut conn = self.get_connection().await?;
        let exists: bool = redis::cmd("EXISTS").arg(key).query_async(&mut conn).await?;
        Ok(exists)
    }

    async fn expire(&self, key: &str, ttl_seconds: u64) -> Result<bool, Self::Error> {
        let mut conn = self.get_connection().await?;
        let result: bool = redis::cmd("EXPIRE").arg(key).arg(ttl_seconds).query_async(&mut conn).await?;
        Ok(result)
    }

    async fn list_push<T: Serialize + std::marker::Sync>(&self, key: &str, value: &T) -> Result<(), Self::Error> {
        let mut conn = self.get_connection().await?;
        let json_str = serde_json::to_string(value)
            .map_err(|e| redis::RedisError::from((redis::ErrorKind::TypeError, "JSON serialization failed", e.to_string())))?;

        redis::cmd("LPUSH").arg(key).arg(json_str).query_async(&mut conn).await
    }

    async fn list_pop<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<Option<T>, Self::Error> {
        let mut conn = self.get_connection().await?;
        let value: Option<String> = redis::cmd("RPOP").arg(key).query_async(&mut conn).await?;

        match value {
            Some(json_str) => {
                let item: T = serde_json::from_str(&json_str)
                    .map_err(|e| redis::RedisError::from((redis::ErrorKind::TypeError, "JSON deserialization failed", e.to_string())))?;
                Ok(Some(item))
            }
            None => Ok(None),
        }
    }

    async fn list_len(&self, key: &str) -> Result<usize, Self::Error> {
        let mut conn = self.get_connection().await?;
        let len: usize = redis::cmd("LLEN").arg(key).query_async(&mut conn).await?;
        Ok(len)
    }

    async fn hash_set<T: Serialize + std::marker::Sync>(&self, key: &str, field: &str, value: &T) -> Result<(), Self::Error> {
        let mut conn = self.get_connection().await?;
        let json_str = serde_json::to_string(value)
            .map_err(|e| redis::RedisError::from((redis::ErrorKind::TypeError, "JSON serialization failed", e.to_string())))?;

        redis::cmd("HSET").arg(key).arg(field).arg(json_str).query_async(&mut conn).await
    }

    async fn hash_get<T: for<'de> Deserialize<'de>>(&self, key: &str, field: &str) -> Result<Option<T>, Self::Error> {
        let mut conn = self.get_connection().await?;
        let value: Option<String> = redis::cmd("HGET").arg(key).arg(field).query_async(&mut conn).await?;

        match value {
            Some(json_str) => {
                let item: T = serde_json::from_str(&json_str)
                    .map_err(|e| redis::RedisError::from((redis::ErrorKind::TypeError, "JSON deserialization failed", e.to_string())))?;
                Ok(Some(item))
            }
            None => Ok(None),
        }
    }

    async fn hash_delete(&self, key: &str, field: &str) -> Result<bool, Self::Error> {
        let mut conn = self.get_connection().await?;
        let count: i32 = redis::cmd("HDEL").arg(key).arg(field).query_async(&mut conn).await?;
        Ok(count > 0)
    }

    async fn hash_exists(&self, key: &str, field: &str) -> Result<bool, Self::Error> {
        let mut conn = self.get_connection().await?;
        let exists: bool = redis::cmd("HEXISTS").arg(key).arg(field).query_async(&mut conn).await?;
        Ok(exists)
    }

    async fn delete_pattern(&self, pattern: &str) -> Result<u64, Self::Error> {
        let mut conn = self.get_connection().await?;
        let keys: Vec<String> = redis::cmd("KEYS").arg(pattern).query_async(&mut conn).await?;

        if keys.is_empty() {
            return Ok(0);
        }

        let mut conn = self.get_connection().await?;
        let count: u64 = redis::cmd("DEL").arg(&keys).query_async(&mut conn).await?;
        Ok(count)
    }
}

// 缓存错误类型
#[derive(Debug, thiserror::Error)]
pub enum CacheError {
    #[error("Redis connection error: {0}")]
    Connection(#[from] redis::RedisError),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Cache key not found: {0}")]
    KeyNotFound(String),

    #[error("Cache operation failed: {0}")]
    OperationFailed(String),
}