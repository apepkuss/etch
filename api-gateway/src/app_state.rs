use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::database::Database;
use crate::cache::Cache;

/// 应用程序状态
#[derive(Clone)]
pub struct AppState {
    /// 应用状态信息
    pub status: Arc<RwLock<AppStatus>>,
    /// 服务配置
    pub config: AppConfig,
    /// 应用统计信息
    pub stats: Arc<RwLock<AppStats>>,
    /// 运行时信息
    pub runtime: Arc<RwLock<RuntimeInfo>>,
    /// 数据库连接
    pub database: Arc<Database>,
    /// Redis缓存
    pub cache: Arc<Cache>,
}

/// 应用状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppStatus {
    pub health: String,
    pub start_time: DateTime<Utc>,
    pub version: String,
    pub environment: String,
}

/// 应用配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub features: FeatureConfig,
}

/// 服务器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub workers: usize,
}

/// 功能配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureConfig {
    pub auth_enabled: bool,
    pub websocket_enabled: bool,
    pub sessions_enabled: bool,
    pub rate_limiting: bool,
}

/// 应用统计信息
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct AppStats {
    pub total_requests: u64,
    pub active_connections: u64,
    pub authenticated_users: u64,
    pub device_count: u64,
    pub session_count: u64,
    pub errors: u64,
    pub last_updated: Option<DateTime<Utc>>,
}

/// 运行时信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeInfo {
    pub uptime_seconds: u64,
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
}

impl AppState {
    pub async fn new() -> Result<Self, anyhow::Error> {
        let config = AppConfig {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8080,
                workers: 4,
            },
            features: FeatureConfig {
                auth_enabled: true,
                websocket_enabled: true,
                sessions_enabled: true,
                rate_limiting: false,
            },
        };

        let status = AppStatus {
            health: "healthy".to_string(),
            start_time: Utc::now(),
            version: "0.1.0".to_string(),
            environment: "development".to_string(),
        };

        // 初始化数据库连接
        let database = Database::new().await?;

        // 运行数据库迁移
        if let Err(e) = database.run_migrations().await {
            tracing::warn!("Database migrations failed: {}", e);
        }

        // 初始化Redis缓存
        let cache = Cache::new().await?;

        Ok(Self {
            status: Arc::new(RwLock::new(status)),
            config,
            stats: Arc::new(RwLock::new(AppStats::default())),
            runtime: Arc::new(RwLock::new(RuntimeInfo {
                uptime_seconds: 0,
                memory_usage_mb: 0.0,
                cpu_usage_percent: 0.0,
            })),
            database: Arc::new(database),
            cache: Arc::new(cache),
        })
    }

    /// 获取应用健康状态
    pub async fn get_health_status(&self) -> AppStatus {
        self.status.read().await.clone()
    }

    /// 更新应用健康状态
    pub async fn update_health_status(&self, health: String) {
        let mut status = self.status.write().await;
        status.health = health;
    }

    /// 获取应用统计信息
    pub async fn get_stats(&self) -> AppStats {
        self.stats.read().await.clone()
    }

    /// 增加请求计数
    pub async fn increment_requests(&self) {
        let mut stats = self.stats.write().await;
        stats.total_requests += 1;
        stats.last_updated = Some(Utc::now());
    }

    /// 更新活跃连接数
    pub async fn update_active_connections(&self, count: u64) {
        let mut stats = self.stats.write().await;
        stats.active_connections = count;
        stats.last_updated = Some(Utc::now());
    }

    /// 更新设备计数
    pub async fn update_device_count(&self, count: u64) {
        let mut stats = self.stats.write().await;
        stats.device_count = count;
        stats.last_updated = Some(Utc::now());
    }

    /// 更新会话计数
    pub async fn update_session_count(&self, count: u64) {
        let mut stats = self.stats.write().await;
        stats.session_count = count;
        stats.last_updated = Some(Utc::now());
    }

    /// 增加错误计数
    pub async fn increment_errors(&self) {
        let mut stats = self.stats.write().await;
        stats.errors += 1;
        stats.last_updated = Some(Utc::now());
    }

    /// 更新运行时信息
    pub async fn update_runtime_info(&self, uptime_seconds: u64, memory_usage_mb: f64, cpu_usage_percent: f64) {
        let mut runtime = self.runtime.write().await;
        runtime.uptime_seconds = uptime_seconds;
        runtime.memory_usage_mb = memory_usage_mb;
        runtime.cpu_usage_percent = cpu_usage_percent;
    }

    /// 获取完整的系统信息
    pub async fn get_system_info(&self) -> SystemInfo {
        let status = self.get_health_status().await;
        let stats = self.get_stats().await;
        let runtime = self.runtime.read().await.clone();

        SystemInfo {
            status,
            config: self.config.clone(),
            stats,
            runtime,
        }
    }
}

/// 系统信息聚合
#[derive(Debug, Serialize)]
pub struct SystemInfo {
    pub status: AppStatus,
    pub config: AppConfig,
    pub stats: AppStats,
    pub runtime: RuntimeInfo,
}