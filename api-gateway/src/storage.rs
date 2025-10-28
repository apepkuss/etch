// 存储层集成 - 数据库和缓存管理
use std::sync::Arc;
use sqlx::{PgPool, postgres::PgPoolOptions};
use redis::Client as RedisClient;
use echo_shared::{DatabaseError, CacheError, RedisCache, CacheOperations};
use anyhow::Result;

// 存储层配置
#[derive(Debug, Clone)]
pub struct StorageConfig {
    pub database_url: String,
    pub redis_url: String,
    pub max_connections: u32,
    pub connection_timeout: u64,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://echo_user:echo_password@localhost:5432/echo_db".to_string()),
            redis_url: std::env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://:redis_password@localhost:6379".to_string()),
            max_connections: std::env::var("DB_MAX_CONNECTIONS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10),
            connection_timeout: std::env::var("DB_CONNECTION_TIMEOUT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(30),
        }
    }
}

// 存储层聚合 - 统一管理数据库和缓存
#[derive(Clone)]
pub struct Storage {
    pub db: Arc<PgPool>,
    pub cache: Arc<RedisCache>,
}

impl Storage {
    /// 初始化存储层
    pub async fn new(config: StorageConfig) -> Result<Self> {
        // 初始化数据库连接池
        let db_pool = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .acquire_timeout(std::time::Duration::from_secs(config.connection_timeout))
            .connect(&config.database_url)
            .await
            .map_err(|e| DatabaseError::Connection(e.to_string()))?;

        // 初始化Redis客户端
        let redis_client = RedisClient::open(&config.redis_url)?;
        let redis_cache = RedisCache::new(&config.redis_url)?;

        // 测试连接
        self::test_connections(&db_pool, &redis_cache).await?;

        Ok(Self {
            db: Arc::new(db_pool),
            cache: Arc::new(redis_cache),
        })
    }

    /// 获取数据库连接池
    pub fn db(&self) -> &PgPool {
        &self.db
    }

    /// 获取缓存客户端
    pub fn cache(&self) -> &RedisCache {
        &self.cache
    }

    /// 健康检查
    pub async fn health_check(&self) -> Result<StorageHealth> {
        let db_healthy = sqlx::query("SELECT 1")
            .fetch_one(self.db.as_ref())
            .await
            .is_ok();

        let cache_healthy = self.cache.exists("health_check").await.is_ok();

        Ok(StorageHealth {
            database: db_healthy,
            cache: cache_healthy,
        })
    }

    /// 关闭存储层连接
    pub async fn close(&self) -> Result<()> {
        self.db.close().await;
        Ok(())
    }
}

// 存储层健康状态
#[derive(Debug, serde::Serialize)]
pub struct StorageHealth {
    pub database: bool,
    pub cache: bool,
}

// 测试存储层连接
async fn test_connections(
    db_pool: &PgPool,
    redis_cache: &RedisCache,
) -> Result<()> {
    // 测试数据库连接
    let db_result = sqlx::query("SELECT version()")
        .fetch_one(db_pool)
        .await;

    if let Err(e) = db_result {
        return Err(DatabaseError::Connection(e.to_string()).into());
    }

    // 测试Redis连接
    let test_key = "connection_test";
    redis_cache.set(test_key, &"test_value", 10).await
        .map_err(|e| CacheError::Connection(e))?;

    let _value: Option<String> = redis_cache.get(test_key).await
        .map_err(|e| CacheError::Connection(e))?;

    redis_cache.delete(test_key).await
        .map_err(|e| CacheError::Connection(e))?;

    Ok(())
}

// 数据库事务辅助函数
pub async fn with_transaction<F, R>(
    pool: &PgPool,
    f: F,
) -> Result<R, DatabaseError>
where
    F: FnOnce(&mut sqlx::Transaction<'_, sqlx::Postgres>) -> Result<R, DatabaseError>,
{
    let mut tx = pool.begin().await
        .map_err(|e| DatabaseError::Connection(e.to_string()))?;

    match f(&mut tx) {
        Ok(result) => {
            tx.commit().await
                .map_err(|e| DatabaseError::Connection(e.to_string()))?;
            Ok(result)
        }
        Err(e) => {
            tx.rollback().await
                .map_err(|rollback_err| {
                    DatabaseError::Connection(format!(
                        "Transaction error: {}, rollback error: {}",
                        e, rollback_err
                    ))
                })?;
            Err(e)
        }
    }
}

// 缓存策略辅助函数
pub struct CacheStrategy;

impl CacheStrategy {
    /// 缓存穿透保护 - 如果缓存未命中，从数据库加载并写入缓存
    pub async fn get_or_set<T, F, Fut>(
        cache: &RedisCache,
        key: &str,
        loader: F,
        ttl_seconds: u64,
    ) -> Result<T, CacheError>
    where
        T: serde::Serialize + for<'de> serde::Deserialize<'de> + Send + Sync + 'static,
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, DatabaseError>> + Send,
    {
        // 尝试从缓存获取
        if let Some(value) = cache.get::<T>(key).await? {
            return Ok(value);
        }

        // 缓存未命中，从数据库加载
        let value = loader().await
            .map_err(|e| CacheError::OperationFailed(e.to_string()))?;

        // 写入缓存
        cache.set(key, &value, ttl_seconds).await?;

        Ok(value)
    }

    /// 批量清理缓存
    pub async fn invalidate_pattern(
        cache: &RedisCache,
        pattern: &str,
    ) -> Result<u64, CacheError> {
        cache.delete_pattern(pattern).await
    }

    /// 清除用户相关的所有缓存
    pub async fn clear_user_cache(
        cache: &RedisCache,
        user_id: &str,
    ) -> Result<u64, CacheError> {
        let patterns = vec![
            &format!("devices:list:{}", user_id),
            &format!("user:session:{}", user_id),
            &format!("user:token:*"), // 需要模式匹配
        ];

        let mut deleted_count = 0;
        for pattern in patterns {
            deleted_count += cache.delete_pattern(pattern).await?;
        }

        Ok(deleted_count)
    }

    /// 清除设备相关的所有缓存
    pub async fn clear_device_cache(
        cache: &RedisCache,
        device_id: &str,
    ) -> Result<u64, CacheError> {
        let patterns = vec![
            &format!("device:status:{}", device_id),
            &format!("device:config:{}", device_id),
        ];

        let mut deleted_count = 0;
        for pattern in patterns {
            deleted_count += cache.delete_pattern(pattern).await?;
        }

        Ok(deleted_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use echo_shared::Device;

    #[tokio::test]
    async fn test_cache_strategy_get_or_set() {
        let config = StorageConfig::default();
        let storage = Storage::new(config).await.unwrap();

        let key = "test_device_list";
        let expected_devices = vec![
            Device {
                id: "test-1".to_string(),
                name: "Test Device 1".to_string(),
                device_type: echo_shared::DeviceType::Speaker,
                status: echo_shared::DeviceStatus::Online,
                location: "Test Room".to_string(),
                firmware_version: "1.0.0".to_string(),
                battery_level: 80,
                volume: 50,
                last_seen: chrono::Utc::now(),
                is_online: true,
                owner: "test-user".to_string(),
            }
        ];

        // 第一次调用 - 应该从数据库加载并缓存
        let devices = CacheStrategy::get_or_set(
            storage.cache(),
            key,
            || async move { Ok(expected_devices.clone()) },
            60,
        ).await.unwrap();

        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].id, "test-1");

        // 第二次调用 - 应该从缓存获取
        let cached_devices = CacheStrategy::get_or_set(
            storage.cache(),
            key,
            || async move { panic!("Should not call loader when cache is hit") },
            60,
        ).await.unwrap();

        assert_eq!(cached_devices.len(), 1);
        assert_eq!(cached_devices[0].id, "test-1");

        // 清理测试数据
        storage.cache().delete(key).await.unwrap();
    }
}