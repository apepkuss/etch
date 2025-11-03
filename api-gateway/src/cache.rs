use std::env;
use anyhow::Result;
use redis::Client as RedisClient;
use tracing::{info, error, warn};
use serde::{Deserialize, Serialize};

/// Redis 缓存连接
#[derive(Clone)]
pub struct Cache {
    client: RedisClient,
}

impl Cache {
    /// 创建新的缓存连接
    pub async fn new() -> Result<Self> {
        let redis_url = env::var("REDIS_URL")
            .unwrap_or_else(|_| "redis://:redis_password@localhost:6379".to_string());

        info!("Connecting to Redis: {}", redis_url);

        let client = RedisClient::open(redis_url)?;

        // 测试连接
        let mut conn = client.get_multiplexed_async_connection().await?;
        let _: Option<String> = redis::cmd("PING").query_async(&mut conn).await?;

        info!("Redis connection established successfully");

        Ok(Cache { client })
    }

    /// 获取连接
    async fn get_connection(&self) -> Result<redis::aio::MultiplexedConnection, redis::RedisError> {
        self.client.get_multiplexed_async_connection().await
    }

    /// 健康检查
    pub async fn health_check(&self) -> Result<bool> {
        match self.get_connection().await {
            Ok(mut conn) => {
                match redis::cmd("PING").query_async::<_, Option<String>>(&mut conn).await {
                    Ok(Some(response)) if response == "PONG" => {
                        info!("Redis health check: OK");
                        Ok(true)
                    }
                    Ok(_) => {
                        warn!("Redis health check: unexpected response");
                        Ok(false)
                    }
                    Err(e) => {
                        error!("Redis health check failed: {}", e);
                        Ok(false)
                    }
                }
            }
            Err(e) => {
                error!("Redis connection failed: {}", e);
                Ok(false)
            }
        }
    }
}

// 基本缓存操作
impl Cache {
    /// 获取缓存值
    pub async fn get<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<Option<T>> {
        let mut conn = self.get_connection().await?;
        let value: Option<String> = redis::cmd("GET").arg(key).query_async(&mut conn).await?;

        match value {
            Some(json_str) => {
                let item: T = serde_json::from_str(&json_str)?;
                Ok(Some(item))
            }
            None => Ok(None),
        }
    }

    /// 设置缓存值（带过期时间）
    pub async fn set<T: Serialize>(&self, key: &str, value: &T, ttl_seconds: u64) -> Result<()> {
        let mut conn = self.get_connection().await?;
        let json_str = serde_json::to_string(value)?;

        redis::cmd("SETEX")
            .arg(key)
            .arg(ttl_seconds)
            .arg(json_str)
            .query_async::<_, ()>(&mut conn)
            .await?;

        Ok(())
    }

    /// 删除缓存值
    pub async fn delete(&self, key: &str) -> Result<bool> {
        let mut conn = self.get_connection().await?;
        let count: i32 = redis::cmd("DEL").arg(key).query_async(&mut conn).await?;
        Ok(count > 0)
    }

    /// 检查键是否存在
    pub async fn exists(&self, key: &str) -> Result<bool> {
        let mut conn = self.get_connection().await?;
        let exists: bool = redis::cmd("EXISTS").arg(key).query_async(&mut conn).await?;
        Ok(exists)
    }

    /// 设置过期时间
    pub async fn expire(&self, key: &str, ttl_seconds: u64) -> Result<bool> {
        let mut conn = self.get_connection().await?;
        let result: bool = redis::cmd("EXPIRE").arg(key).arg(ttl_seconds).query_async(&mut conn).await?;
        Ok(result)
    }
}

// 用户相关缓存操作
impl Cache {
    /// 生成用户会话缓存键
    pub fn user_session_key(user_id: &str) -> String {
        format!("user:session:{}", user_id)
    }

    /// 生成用户Token缓存键
    pub fn user_token_key(token: &str) -> String {
        format!("user:token:{}", token)
    }

    /// 缓存用户会话
    pub async fn cache_user_session(&self, user_id: &str, session_data: &UserSessionCache, ttl_seconds: u64) -> Result<()> {
        let key = Self::user_session_key(user_id);
        self.set(&key, session_data, ttl_seconds).await
    }

    /// 获取用户会话
    pub async fn get_user_session(&self, user_id: &str) -> Result<Option<UserSessionCache>> {
        let key = Self::user_session_key(user_id);
        self.get(&key).await
    }

    /// 缓存用户Token
    pub async fn cache_user_token(&self, token: &str, user_id: &str, ttl_seconds: u64) -> Result<()> {
        let key = Self::user_token_key(token);
        self.set(&key, &user_id.to_string(), ttl_seconds).await
    }

    /// 根据Token获取用户ID
    pub async fn get_user_by_token(&self, token: &str) -> Result<Option<String>> {
        let key = Self::user_token_key(token);
        self.get(&key).await
    }
}

// 设备相关缓存操作
impl Cache {
    /// 生成设备状态缓存键
    pub fn device_status_key(device_id: &str) -> String {
        format!("device:status:{}", device_id)
    }

    /// 生成设备配置缓存键
    pub fn device_config_key(device_id: &str) -> String {
        format!("device:config:{}", device_id)
    }

    /// 缓存设备状态
    pub async fn cache_device_status(&self, device_id: &str, status: &DeviceStatusCache, ttl_seconds: u64) -> Result<()> {
        let key = Self::device_status_key(device_id);
        self.set(&key, status, ttl_seconds).await
    }

    /// 获取设备状态
    pub async fn get_device_status(&self, device_id: &str) -> Result<Option<DeviceStatusCache>> {
        let key = Self::device_status_key(device_id);
        self.get(&key).await
    }

    /// 缓存设备配置
    pub async fn cache_device_config(&self, device_id: &str, config: &DeviceConfigCache, ttl_seconds: u64) -> Result<()> {
        let key = Self::device_config_key(device_id);
        self.set(&key, config, ttl_seconds).await
    }

    /// 获取设备配置
    pub async fn get_device_config(&self, device_id: &str) -> Result<Option<DeviceConfigCache>> {
        let key = Self::device_config_key(device_id);
        self.get(&key).await
    }
}

// 清理相关操作
impl Cache {
    /// 清理用户相关的所有缓存
    pub async fn clear_user_cache(&self, user_id: &str) -> Result<u64> {
        let session_key = Self::user_session_key(user_id);
        let _token_pattern = format!("user:token:*");

        // 删除会话缓存
        self.delete(&session_key).await?;

        // 这里简化实现，实际应该使用SCAN来避免KEYS命令的性能问题
        // 暂时跳过批量删除token
        Ok(1)
    }

    /// 清理设备相关的所有缓存
    pub async fn clear_device_cache(&self, device_id: &str) -> Result<u64> {
        let status_key = Self::device_status_key(device_id);
        let config_key = Self::device_config_key(device_id);

        let mut deleted = 0;
        if self.delete(&status_key).await? {
            deleted += 1;
        }
        if self.delete(&config_key).await? {
            deleted += 1;
        }

        Ok(deleted)
    }
}

// 缓存数据结构定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSessionCache {
    pub user_id: String,
    pub username: String,
    pub role: String,
    pub permissions: Vec<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceStatusCache {
    pub device_id: String,
    pub status: String,
    pub battery_level: Option<i32>,
    pub volume: Option<i32>,
    pub location: Option<String>,
    pub last_seen: chrono::DateTime<chrono::Utc>,
    pub is_online: bool,
}

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
    pub updated_at: chrono::DateTime<chrono::Utc>,
}