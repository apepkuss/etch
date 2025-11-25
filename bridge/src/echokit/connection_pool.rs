use anyhow::{Context, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tracing::{debug, error, info, warn};
use sqlx::PgPool;

use crate::echokit_client::EchoKitConnectionManager;

/// EchoKit 连接池 - 管理多个 EchoKit Server 的连接
///
/// 核心设计：
/// - 键是 echokit_server_url (如 "wss://indie.echokit.dev/ws/{device_id}")
/// - 值是对应的 EchoKitConnectionManager
/// - 相同 URL 的设备共享同一个连接
/// - 懒加载：只在需要时创建连接
pub struct EchoKitConnectionPool {
    /// 核心存储：echokit_server_url -> EchoKitConnectionManager
    connections: Arc<RwLock<HashMap<String, Arc<EchoKitConnectionManager>>>>,

    /// 数据库连接池，用于查询设备的 echokit_server_url
    db_pool: Arc<PgPool>,

    /// 回调通道（从 main.rs 传入，所有连接共享）
    audio_callback: mpsc::UnboundedSender<(String, Vec<u8>)>,
    asr_callback: mpsc::UnboundedSender<(String, String)>,
    response_callback: mpsc::UnboundedSender<(String, String)>,
    raw_message_callback: mpsc::UnboundedSender<(String, Vec<u8>)>,
}

impl EchoKitConnectionPool {
    /// 创建新的连接池（HashMap 初始为空，懒加载）
    pub fn new(
        db_pool: Arc<PgPool>,
        audio_callback: mpsc::UnboundedSender<(String, Vec<u8>)>,
        asr_callback: mpsc::UnboundedSender<(String, String)>,
        response_callback: mpsc::UnboundedSender<(String, String)>,
        raw_message_callback: mpsc::UnboundedSender<(String, Vec<u8>)>,
    ) -> Self {
        info!("🔧 Creating EchoKitConnectionPool (lazy loading mode)");

        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            db_pool,
            audio_callback,
            asr_callback,
            response_callback,
            raw_message_callback,
        }
    }

    /// 根据设备 ID 获取对应的 EchoKit 连接管理器
    pub async fn get_connection_for_device(
        &self,
        device_id: &str,
    ) -> Result<Arc<EchoKitConnectionManager>> {
        // 步骤 1：从数据库查询设备的 echokit_server_url（模板格式）
        let echokit_url_template = self.get_device_echokit_url(device_id).await?;

        // 步骤 2：将 {device_id} 占位符替换为实际的设备 ID
        let echokit_url = echokit_url_template.replace("{device_id}", device_id);

        debug!("📝 URL template: {} -> resolved: {}", echokit_url_template, echokit_url);

        // 步骤 3：使用替换后的完整 URL 获取或创建连接
        self.get_or_create_connection(&echokit_url).await
    }

    /// 获取或创建指定 URL 的连接管理器（核心逻辑）
    ///
    /// 使用双重检查锁定模式避免并发重复创建
    pub async fn get_or_create_connection(
        &self,
        echokit_url: &str,
    ) -> Result<Arc<EchoKitConnectionManager>> {
        // 🔍 第一次检查：读锁，检查连接是否已存在
        {
            let connections = self.connections.read().await;
            if let Some(manager) = connections.get(echokit_url) {
                debug!("♻️ Reusing existing EchoKit connection for {}", echokit_url);
                return Ok(manager.clone());
            }
        } // 读锁自动释放

        // 🔒 第二次检查：写锁，双重检查避免并发重复创建
        let mut connections = self.connections.write().await;

        // 再次检查（可能其他线程已经创建了）
        if let Some(manager) = connections.get(echokit_url) {
            debug!("♻️ Connection created by another task for {}", echokit_url);
            return Ok(manager.clone());
        }

        // 🆕 创建新的连接管理器
        info!("🔌 Creating new EchoKit connection for {}", echokit_url);

        let manager = Arc::new(EchoKitConnectionManager::new_with_all_callbacks(
            echokit_url.to_string(),
            self.audio_callback.clone(),
            self.asr_callback.clone(),
            self.response_callback.clone(),
            self.raw_message_callback.clone(),
        ));

        // 🚀 启动连接（后台异步连接）
        manager.start().await
            .with_context(|| format!("Failed to start EchoKit connection for {}", echokit_url))?;

        // 🔌 预先连接到 EchoKit Server
        info!("🔌 Pre-connecting to EchoKit Server: {}", echokit_url);
        if let Err(e) = manager.get_client().connect().await {
            warn!("⚠️ Failed to pre-connect to EchoKit Server {}: {}. Will retry on first session.", echokit_url, e);
        } else {
            info!("✅ Pre-connected to EchoKit Server: {}", echokit_url);
        }

        // 💾 存储到 HashMap
        connections.insert(echokit_url.to_string(), manager.clone());

        info!("✅ New EchoKit connection established and cached for {}", echokit_url);
        info!("📊 Total EchoKit connections in pool: {}", connections.len());

        Ok(manager)
    }

    /// 从数据库查询设备的 echokit_server_url
    ///
    /// 注意：数据库约束保证 echokit_server_url 字段不会是 NULL
    async fn get_device_echokit_url(&self, device_id: &str) -> Result<String> {
        let result = sqlx::query!(
            "SELECT echokit_server_url FROM devices WHERE id = $1",
            device_id
        )
        .fetch_optional(&*self.db_pool)
        .await
        .with_context(|| format!("Failed to query device {} from database", device_id))?;

        match result {
            Some(record) => {
                // 数据库字段有 NOT NULL 约束，直接使用
                let url = record.echokit_server_url;
                info!("📍 Device {} using EchoKit URL: {}", device_id, url);
                Ok(url)
            }
            None => {
                // 设备不存在于数据库
                anyhow::bail!("Device {} not found in database", device_id)
            }
        }
    }

    /// 获取当前活跃的连接数量（用于监控）
    pub async fn get_connection_count(&self) -> usize {
        self.connections.read().await.len()
    }

    /// 获取所有连接的 URL 列表（用于调试）
    pub async fn get_connection_urls(&self) -> Vec<String> {
        self.connections.read().await.keys().cloned().collect()
    }

    /// 关闭指定 URL 的连接（用于清理）
    pub async fn close_connection(&self, echokit_url: &str) -> Result<()> {
        let mut connections = self.connections.write().await;

        if let Some(manager) = connections.remove(echokit_url) {
            info!("🔌 Closing EchoKit connection for {}", echokit_url);
            // 断开连接
            if let Err(e) = manager.get_client().disconnect().await {
                warn!("⚠️ Error disconnecting from {}: {}", echokit_url, e);
            }
            drop(manager);
            info!("📊 Remaining EchoKit connections: {}", connections.len());
        } else {
            debug!("⚠️ Connection for {} not found in pool", echokit_url);
        }

        Ok(())
    }

    /// 关闭所有连接（用于服务关闭）
    pub async fn close_all_connections(&self) -> Result<()> {
        let mut connections = self.connections.write().await;

        info!("🔌 Closing all {} EchoKit connections", connections.len());

        for (url, manager) in connections.drain() {
            info!("🔌 Closing connection: {}", url);
            if let Err(e) = manager.get_client().disconnect().await {
                warn!("⚠️ Error disconnecting from {}: {}", url, e);
            }
        }

        info!("✅ All EchoKit connections closed");
        Ok(())
    }
}

impl Drop for EchoKitConnectionPool {
    fn drop(&mut self) {
        info!("🔌 EchoKitConnectionPool is being dropped");
    }
}
