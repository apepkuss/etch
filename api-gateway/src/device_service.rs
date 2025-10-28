// 设备管理服务 - 数据访问层
use std::sync::Arc;
use sqlx::{PgPool, Row};
use echo_shared::{
    Device, DeviceType, DeviceStatus, DeviceRecord, DeviceFilter,
    DatabaseError, CreateDeviceRequest, UpdateDeviceRequest,
    RedisCache, CacheStrategy, ttl, DeviceStatusCache, DeviceConfigCache,
};
use anyhow::Result;
use uuid::Uuid;

// 设备服务
#[derive(Clone)]
pub struct DeviceService {
    db: Arc<PgPool>,
    cache: Arc<RedisCache>,
}

impl DeviceService {
    pub fn new(db: Arc<PgPool>, cache: Arc<RedisCache>) -> Self {
        Self { db, cache }
    }

    /// 获取用户的设备列表
    pub async fn get_user_devices(
        &self,
        user_id: &str,
        filter: Option<DeviceFilter>,
    ) -> Result<Vec<Device>> {
        let cache_key = format!("devices:list:{}:{:?}", user_id, filter);

        // 尝试从缓存获取
        if let Some(devices) = self.cache.get::<Vec<Device>>(&cache_key).await? {
            return Ok(devices);
        }

        // 从数据库查询
        let query = r#"
            SELECT id, name, device_type, status, location, firmware_version,
                   battery_level, volume, last_seen, is_online, owner_id,
                   created_at, updated_at, config
            FROM devices
            WHERE owner_id = $1
        "#;

        let mut builder = sqlx::query_builder::QueryBuilder::new(query);
        let mut bind_count = 1;

        if let Some(f) = &filter {
            if let Some(device_type) = &f.device_type {
                bind_count += 1;
                builder.push(" AND device_type = $");
                builder.push_bind(bind_count);
                builder.push(" ");
            }
            if let Some(status) = &f.status {
                bind_count += 1;
                builder.push(" AND status = $");
                builder.push_bind(bind_count);
                builder.push(" ");
            }
            if let Some(is_online) = f.is_online {
                bind_count += 1;
                builder.push(" AND is_online = $");
                builder.push_bind(bind_count);
                builder.push(" ");
            }
        }

        builder.push(" ORDER BY last_seen DESC");

        if let Some(limit) = filter.as_ref().and_then(|f| f.limit) {
            bind_count += 1;
            builder.push(" LIMIT $");
            builder.push_bind(bind_count);
            builder.push(" ");
        }

        if let Some(offset) = filter.as_ref().and_then(|f| f.offset) {
            bind_count += 1;
            builder.push(" OFFSET $");
            builder.push_bind(bind_count);
        }

        // 构建查询需要处理类型转换
        let devices = if let Some(f) = filter {
            let mut query = sqlx::query_as!(
                DeviceRecord,
                r#"
                SELECT id, name, device_type as "device_type: DeviceType", status as "status: DeviceStatus",
                       location, firmware_version, battery_level, volume, last_seen, is_online,
                       owner_id, created_at, updated_at, config
                FROM devices
                WHERE owner_id = $1
                "#,
            );

            if let Some(device_type) = f.device_type {
                query = query.bind(device_type);
            }
            if let Some(status) = f.status {
                query = query.bind(status);
            }
            if let Some(is_online) = f.is_online {
                query = query.bind(is_online);
            }

            query = query.order_by("last_seen DESC");

            if let Some(limit) = f.limit {
                query = query.limit(limit);
            }
            if let Some(offset) = f.offset {
                query = query.offset(offset);
            }

            query
                .bind(user_id)
                .fetch_all(self.db.as_ref())
                .await
                .map_err(|e| DatabaseError::Connection(e.to_string()))?
        } else {
            sqlx::query_as!(
                DeviceRecord,
                r#"
                SELECT id, name, device_type as "device_type: DeviceType", status as "status: DeviceStatus",
                       location, firmware_version, battery_level, volume, last_seen, is_online,
                       owner_id, created_at, updated_at, config
                FROM devices
                WHERE owner_id = $1
                ORDER BY last_seen DESC
                "#,
                user_id
            )
            .fetch_all(self.db.as_ref())
            .await
            .map_err(|e| DatabaseError::Connection(e.to_string()))?
        };

        let result: Vec<Device> = devices.into_iter().map(|record| self.record_to_device(record)).collect();

        // 缓存结果
        self.cache.set(&cache_key, &result, ttl::DEVICE_LIST).await
            .map_err(|e| DatabaseError::Connection(e.to_string()))?;

        Ok(result)
    }

    /// 根据ID获取设备
    pub async fn get_device_by_id(&self, device_id: &str) -> Result<Option<Device>> {
        let cache_key = format!("device:{}", device_id);

        // 尝试从缓存获取
        if let Some(device) = self.cache.get::<Device>(&cache_key).await? {
            return Ok(Some(device));
        }

        // 从数据库查询
        let record = sqlx::query_as!(
            DeviceRecord,
            r#"
            SELECT id, name, device_type as "device_type: DeviceType", status as "status: DeviceStatus",
                   location, firmware_version, battery_level, volume, last_seen, is_online,
                   owner_id, created_at, updated_at, config
            FROM devices
            WHERE id = $1
            "#,
            Uuid::parse_str(device_id)
                .map_err(|_| DatabaseError::InvalidInput("Invalid device ID".to_string()))?
        )
        .fetch_optional(self.db.as_ref())
        .await
        .map_err(|e| DatabaseError::Connection(e.to_string()))?;

        if let Some(record) = record {
            let device = self.record_to_device(record);

            // 缓存结果
            self.cache.set(&cache_key, &device, ttl::DEVICE_STATUS).await
                .map_err(|e| DatabaseError::Connection(e.to_string()))?;

            Ok(Some(device))
        } else {
            Ok(None)
        }
    }

    /// 创建新设备
    pub async fn create_device(
        &self,
        request: CreateDeviceRequest,
        owner_id: &str,
    ) -> Result<Device> {
        let device_id = Uuid::new_v4();

        let record = sqlx::query_as!(
            DeviceRecord,
            r#"
            INSERT INTO devices (id, name, device_type, location, firmware_version, owner_id)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, name, device_type as "device_type: DeviceType", status as "status: DeviceStatus",
                      location, firmware_version, battery_level, volume, last_seen, is_online,
                      owner_id, created_at, updated_at, config
            "#,
            device_id,
            request.name,
            request.device_type as DeviceType,
            request.location,
            request.firmware_version,
            Uuid::parse_str(owner_id)
                .map_err(|_| DatabaseError::InvalidInput("Invalid owner ID".to_string()))?
        )
        .fetch_one(self.db.as_ref())
        .await
        .map_err(|e| DatabaseError::Connection(e.to_string()))?;

        let device = self.record_to_device(record);

        // 清除用户设备列表缓存
        let _ = CacheStrategy::clear_user_cache(self.cache.as_ref(), owner_id).await;

        Ok(device)
    }

    /// 更新设备信息
    pub async fn update_device(
        &self,
        device_id: &str,
        request: UpdateDeviceRequest,
    ) -> Result<Option<Device>> {
        let device_uuid = Uuid::parse_str(device_id)
            .map_err(|_| DatabaseError::InvalidInput("Invalid device ID".to_string()))?;

        let record = sqlx::query_as!(
            DeviceRecord,
            r#"
            UPDATE devices
            SET name = COALESCE($1, name),
                location = COALESCE($2, location),
                volume = COALESCE($3, volume),
                config = COALESCE($4, config),
                updated_at = NOW()
            WHERE id = $5
            RETURNING id, name, device_type as "device_type: DeviceType", status as "status: DeviceStatus",
                      location, firmware_version, battery_level, volume, last_seen, is_online,
                      owner_id, created_at, updated_at, config
            "#,
            request.name,
            request.location,
            request.volume,
            request.config,
            device_uuid
        )
        .fetch_optional(self.db.as_ref())
        .await
        .map_err(|e| DatabaseError::Connection(e.to_string()))?;

        if let Some(record) = record {
            let device = self.record_to_device(record);

            // 清除相关缓存
            let cache_key = format!("device:{}", device_id);
            let _ = self.cache.delete(&cache_key).await;
            let _ = CacheStrategy::clear_device_cache(self.cache.as_ref(), device_id).await;
            let _ = CacheStrategy::clear_user_cache(self.cache.as_ref(), &device.owner).await;

            Ok(Some(device))
        } else {
            Ok(None)
        }
    }

    /// 更新设备状态
    pub async fn update_device_status(
        &self,
        device_id: &str,
        status: DeviceStatus,
        battery_level: Option<i32>,
        volume: Option<i32>,
        last_seen: Option<chrono::DateTime<chrono::Utc>>,
        is_online: Option<bool>,
    ) -> Result<bool> {
        let device_uuid = Uuid::parse_str(device_id)
            .map_err(|_| DatabaseError::InvalidInput("Invalid device ID".to_string()))?;

        let result = sqlx::query!(
            r#"
            UPDATE devices
            SET status = $1,
                battery_level = COALESCE($2, battery_level),
                volume = COALESCE($3, volume),
                last_seen = COALESCE($4, NOW()),
                is_online = COALESCE($5, is_online),
                updated_at = NOW()
            WHERE id = $6
            "#,
            status as DeviceStatus,
            battery_level,
            volume,
            last_seen,
            is_online,
            device_uuid
        )
        .execute(self.db.as_ref())
        .await
        .map_err(|e| DatabaseError::Connection(e.to_string()))?;

        let updated = result.rows_affected() > 0;

        if updated {
            // 更新状态缓存
            let status_cache = DeviceStatusCache {
                device_id: device_id.to_string(),
                status: status.clone(),
                battery_level,
                volume,
                location: None, // 可以从数据库获取
                last_seen: last_seen.unwrap_or_else(chrono::Utc::now),
                is_online: is_online.unwrap_or(false),
            };

            let cache_key = format!("device:status:{}", device_id);
            let _ = self.cache.set(&cache_key, &status_cache, ttl::DEVICE_STATUS).await;

            // 清除设备详情缓存
            let device_cache_key = format!("device:{}", device_id);
            let _ = self.cache.delete(&device_cache_key).await;
        }

        Ok(updated)
    }

    /// 删除设备
    pub async fn delete_device(&self, device_id: &str, owner_id: &str) -> Result<bool> {
        let device_uuid = Uuid::parse_str(device_id)
            .map_err(|_| DatabaseError::InvalidInput("Invalid device ID".to_string()))?;

        // 获取设备信息用于缓存清理
        let device = self.get_device_by_id(device_id).await?;

        let result = sqlx::query!(
            "DELETE FROM devices WHERE id = $1 AND owner_id = $2",
            device_uuid,
            Uuid::parse_str(owner_id)
                .map_err(|_| DatabaseError::InvalidInput("Invalid owner ID".to_string()))?
        )
        .execute(self.db.as_ref())
        .await
        .map_err(|e| DatabaseError::Connection(e.to_string()))?;

        let deleted = result.rows_affected() > 0;

        if deleted {
            // 清除所有相关缓存
            let _ = self.cache.delete(&format!("device:{}", device_id)).await;
            let _ = CacheStrategy::clear_device_cache(self.cache.as_ref(), device_id).await;
            let _ = CacheStrategy::clear_user_cache(self.cache.as_ref(), owner_id).await;
        }

        Ok(deleted)
    }

    /// 检查用户是否有设备权限
    pub async fn check_device_permission(
        &self,
        user_id: &str,
        device_id: &str,
    ) -> Result<bool> {
        let record = sqlx::query!(
            "SELECT owner_id FROM devices WHERE id = $1",
            Uuid::parse_str(device_id)
                .map_err(|_| DatabaseError::InvalidInput("Invalid device ID".to_string()))?
        )
        .fetch_optional(self.db.as_ref())
        .await
        .map_err(|e| DatabaseError::Connection(e.to_string()))?;

        match record {
            Some(device) => {
                let owner_id = device.owner_id.to_string();
                Ok(owner_id == user_id)
            }
            None => Ok(false),
        }
    }

    /// 获取设备统计信息
    pub async fn get_device_stats(&self, user_id: &str) -> Result<DeviceStats> {
        let stats = sqlx::query!(
            r#"
            SELECT
                COUNT(*) as total_count,
                COUNT(CASE WHEN is_online = TRUE THEN 1 END) as online_count,
                COUNT(CASE WHEN device_type = 'speaker' THEN 1 END) as speaker_count,
                COUNT(CASE WHEN device_type = 'display' THEN 1 END) as display_count,
                COUNT(CASE WHEN device_type = 'hub' THEN 1 END) as hub_count,
                MAX(last_seen) as last_activity
            FROM devices
            WHERE owner_id = $1
            "#,
            Uuid::parse_str(user_id)
                .map_err(|_| DatabaseError::InvalidInput("Invalid user ID".to_string()))?
        )
        .fetch_one(self.db.as_ref())
        .await
        .map_err(|e| DatabaseError::Connection(e.to_string()))?;

        Ok(DeviceStats {
            total_devices: stats.total_count.unwrap_or(0) as i64,
            online_devices: stats.online_count.unwrap_or(0) as i64,
            speaker_count: stats.speaker_count.unwrap_or(0) as i64,
            display_count: stats.display_count.unwrap_or(0) as i64,
            hub_count: stats.hub_count.unwrap_or(0) as i64,
            last_device_activity: stats.last_activity,
        })
    }

    // 辅助方法：将数据库记录转换为Device结构
    fn record_to_device(&self, record: DeviceRecord) -> Device {
        Device {
            id: record.id.to_string(),
            name: record.name,
            device_type: record.device_type,
            status: record.status,
            location: record.location,
            firmware_version: record.firmware_version,
            battery_level: record.battery_level,
            volume: record.volume,
            last_seen: record.last_seen,
            is_online: record.is_online,
            owner: record.owner_id.to_string(),
        }
    }
}

// 设备统计信息
#[derive(Debug, serde::Serialize)]
pub struct DeviceStats {
    pub total_devices: i64,
    pub online_devices: i64,
    pub speaker_count: i64,
    pub display_count: i64,
    pub hub_count: i64,
    pub last_device_activity: Option<chrono::DateTime<chrono::Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use echo_shared::DeviceType;

    #[tokio::test]
    async fn test_device_crud() {
        // 这里需要模拟数据库连接，实际测试需要test database
        // 暂时跳过实际测试
    }
}