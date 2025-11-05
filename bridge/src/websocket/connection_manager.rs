use axum::extract::ws::{Message, WebSocket};
use futures_util::stream::{SplitSink, SplitStream};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info};
use axum::body::Bytes;

pub type WsSender = Arc<RwLock<SplitSink<WebSocket, Message>>>;

/// 设备连接管理器
pub struct DeviceConnectionManager {
    /// device_id -> WebSocket sender
    connections: Arc<RwLock<HashMap<String, WsSender>>>,

    /// session_id -> device_id 映射
    session_device_map: Arc<RwLock<HashMap<String, String>>>,

    /// device_id -> 最后心跳时间
    last_heartbeat: Arc<RwLock<HashMap<String, chrono::DateTime<chrono::Utc>>>>,
}

impl DeviceConnectionManager {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            session_device_map: Arc::new(RwLock::new(HashMap::new())),
            last_heartbeat: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 注册设备连接
    pub async fn register_device(
        &self,
        device_id: String,
        sender: SplitSink<WebSocket, Message>,
    ) -> anyhow::Result<()> {
        let mut connections = self.connections.write().await;
        connections.insert(device_id.clone(), Arc::new(RwLock::new(sender)));

        // 更新心跳时间
        let mut heartbeats = self.last_heartbeat.write().await;
        heartbeats.insert(device_id.clone(), chrono::Utc::now());

        info!("Device {} registered, total connections: {}", device_id, connections.len());
        Ok(())
    }

    /// 移除设备连接
    pub async fn remove_device(&self, device_id: &str) -> anyhow::Result<()> {
        let mut connections = self.connections.write().await;
        connections.remove(device_id);

        let mut heartbeats = self.last_heartbeat.write().await;
        heartbeats.remove(device_id);

        // 清理该设备的所有会话映射
        let mut map = self.session_device_map.write().await;
        map.retain(|_, dev_id| dev_id != device_id);

        info!("Device {} removed, remaining connections: {}", device_id, connections.len());
        Ok(())
    }

    /// 绑定会话到设备
    pub async fn bind_session(
        &self,
        session_id: String,
        device_id: String,
    ) -> anyhow::Result<()> {
        let mut map = self.session_device_map.write().await;
        map.insert(session_id.clone(), device_id.clone());
        debug!("Session {} bound to device {}", session_id, device_id);
        Ok(())
    }

    /// 解绑会话
    pub async fn unbind_session(&self, session_id: &str) -> anyhow::Result<()> {
        let mut map = self.session_device_map.write().await;
        map.remove(session_id);
        debug!("Session {} unbound", session_id);
        Ok(())
    }

    /// 根据会话ID推送音频（二进制）
    pub async fn push_audio_by_session(
        &self,
        session_id: &str,
        audio_data: Vec<u8>,
    ) -> anyhow::Result<()> {
        // 查找设备ID
        let device_id = {
            let map = self.session_device_map.read().await;
            map.get(session_id)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Session {} not found", session_id))?
        };

        // 推送音频
        self.push_audio_to_device(&device_id, audio_data).await
    }

    /// 直接推送音频到设备（二进制）
    pub async fn push_audio_to_device(
        &self,
        device_id: &str,
        audio_data: Vec<u8>,
    ) -> anyhow::Result<()> {
        let connections = self.connections.read().await;
        let sender = connections
            .get(device_id)
            .ok_or_else(|| anyhow::anyhow!("Device {} not connected", device_id))?;

        use futures_util::SinkExt;
        sender.write().await.send(Message::Binary(Bytes::from(audio_data))).await?;
        debug!("Pushed audio to device {}", device_id);
        Ok(())
    }

    /// 发送文本消息到设备
    pub async fn send_text(
        &self,
        device_id: &str,
        text: &str,
    ) -> anyhow::Result<()> {
        let connections = self.connections.read().await;
        let sender = connections
            .get(device_id)
            .ok_or_else(|| anyhow::anyhow!("Device {} not connected", device_id))?;

        use futures_util::SinkExt;
        sender.write().await.send(Message::Text(text.to_string().into())).await?;
        debug!("Sent text message to device {}", device_id);
        Ok(())
    }

    /// 响应 Pong
    pub async fn send_pong(
        &self,
        device_id: &str,
        data: Vec<u8>,
    ) -> anyhow::Result<()> {
        let connections = self.connections.read().await;
        let sender = connections
            .get(device_id)
            .ok_or_else(|| anyhow::anyhow!("Device {} not connected", device_id))?;

        use futures_util::SinkExt;
        sender.write().await.send(Message::Pong(Bytes::from(data))).await?;

        // 更新心跳时间
        let mut heartbeats = self.last_heartbeat.write().await;
        heartbeats.insert(device_id.to_string(), chrono::Utc::now());

        Ok(())
    }

    /// 更新心跳时间
    pub async fn update_heartbeat(&self, device_id: &str) {
        let mut heartbeats = self.last_heartbeat.write().await;
        heartbeats.insert(device_id.to_string(), chrono::Utc::now());
    }

    /// 获取在线设备数量
    pub async fn get_online_count(&self) -> usize {
        let connections = self.connections.read().await;
        connections.len()
    }

    /// 获取活跃会话数量
    pub async fn get_active_sessions_count(&self) -> usize {
        let map = self.session_device_map.read().await;
        map.len()
    }

    /// 检查设备是否在线
    pub async fn is_device_online(&self, device_id: &str) -> bool {
        let connections = self.connections.read().await;
        connections.contains_key(device_id)
    }

    /// 获取过期设备（用于心跳检测）
    pub async fn get_stale_devices(&self, timeout_seconds: i64) -> Vec<String> {
        let now = chrono::Utc::now();
        let timeout_duration = chrono::Duration::seconds(timeout_seconds);

        let heartbeats = self.last_heartbeat.read().await;
        let mut stale = Vec::new();

        for (device_id, last_time) in heartbeats.iter() {
            let duration = now.signed_duration_since(*last_time);
            if duration > timeout_duration {
                stale.push(device_id.clone());
            }
        }

        stale
    }
}
