use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use tracing::{debug, info, warn};

use super::connection_manager::DeviceConnectionManager;
use super::session_manager::SessionManager;

/// 心跳检测配置
#[derive(Debug, Clone)]
pub struct HeartbeatConfig {
    /// 心跳检测间隔（秒）
    pub check_interval_secs: u64,
    /// 心跳超时阈值（秒）
    pub timeout_threshold_secs: i64,
    /// 启用自动断连
    pub auto_disconnect: bool,
}

impl Default for HeartbeatConfig {
    fn default() -> Self {
        Self {
            check_interval_secs: 30,
            timeout_threshold_secs: 90, // 3 * 30秒
            auto_disconnect: true,
        }
    }
}

/// 心跳检测服务
pub struct HeartbeatMonitor {
    connection_manager: Arc<DeviceConnectionManager>,
    session_manager: Arc<SessionManager>,
    config: HeartbeatConfig,
}

impl HeartbeatMonitor {
    pub fn new(
        connection_manager: Arc<DeviceConnectionManager>,
        session_manager: Arc<SessionManager>,
        config: HeartbeatConfig,
    ) -> Self {
        Self {
            connection_manager,
            session_manager,
            config,
        }
    }

    /// 启动心跳监控
    pub async fn start(self: Arc<Self>) {
        info!(
            "Starting heartbeat monitor with interval={}s, timeout={}s",
            self.config.check_interval_secs, self.config.timeout_threshold_secs
        );

        let mut interval = time::interval(Duration::from_secs(self.config.check_interval_secs));

        loop {
            interval.tick().await;

            if let Err(e) = self.check_heartbeats().await {
                warn!("Heartbeat check error: {}", e);
            }
        }
    }

    /// 检查所有设备心跳
    async fn check_heartbeats(&self) -> anyhow::Result<()> {
        let stale_devices = self
            .connection_manager
            .get_stale_devices(self.config.timeout_threshold_secs)
            .await;

        if stale_devices.is_empty() {
            debug!("All devices heartbeat normal");
            return Ok(());
        }

        info!("Found {} stale devices", stale_devices.len());

        for device_id in stale_devices {
            warn!("Device {} heartbeat timeout", device_id);

            // 标记会话超时
            if let Err(e) = self.handle_timeout_device(&device_id).await {
                warn!("Failed to handle timeout device {}: {}", device_id, e);
            }

            // 自动断连
            if self.config.auto_disconnect {
                if let Err(e) = self.connection_manager.remove_device(&device_id).await {
                    warn!("Failed to remove timeout device {}: {}", device_id, e);
                }
            }
        }

        Ok(())
    }

    /// 处理超时设备
    async fn handle_timeout_device(&self, device_id: &str) -> anyhow::Result<()> {
        // 获取设备关联的会话
        let sessions = self
            .session_manager
            .get_sessions_by_device(device_id)
            .await;

        for session_id in sessions {
            info!("Marking session {} as timeout", session_id);
            self.session_manager
                .mark_timeout(&session_id)
                .await?;
        }

        Ok(())
    }

    /// 清理超时会话
    pub async fn cleanup_timeout_sessions(&self) -> anyhow::Result<usize> {
        let timeout_secs = self.config.timeout_threshold_secs;
        let cleaned = self
            .session_manager
            .cleanup_timeout_sessions(timeout_secs)
            .await;

        if cleaned > 0 {
            info!("Cleaned {} timeout sessions", cleaned);
        }

        Ok(cleaned)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_heartbeat_config() {
        let config = HeartbeatConfig::default();
        assert_eq!(config.check_interval_secs, 30);
        assert_eq!(config.timeout_threshold_secs, 90);
        assert!(config.auto_disconnect);
    }

    #[tokio::test]
    async fn test_heartbeat_monitor_creation() {
        let conn_mgr = Arc::new(DeviceConnectionManager::new());
        let session_mgr = Arc::new(SessionManager::new());
        let config = HeartbeatConfig::default();

        let monitor = HeartbeatMonitor::new(conn_mgr, session_mgr, config);
        assert!(Arc::strong_count(&monitor.connection_manager) >= 1);
    }
}
