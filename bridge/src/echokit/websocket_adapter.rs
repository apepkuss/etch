use anyhow::{Context, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tracing::{debug, error, info, warn};

use crate::echokit_client::EchoKitClient;
use crate::websocket::connection_manager::DeviceConnectionManager;
use echo_shared::{AudioFormat, EchoKitConfig};

/// EchoKit 会话适配器 - 负责 Bridge Session 和 EchoKit 的集成
pub struct EchoKitSessionAdapter {
    /// EchoKit 客户端
    echokit_client: Arc<EchoKitClient>,
    /// 设备连接管理器（用于发送音频到设备）
    connection_manager: Arc<DeviceConnectionManager>,
    /// Session 映射: bridge_session_id -> (device_id, echokit_session_id)
    session_mapping: Arc<RwLock<HashMap<String, (String, String)>>>,
    /// 音频接收通道
    audio_receiver: Arc<RwLock<Option<mpsc::UnboundedReceiver<(String, Vec<u8>)>>>>,
}

impl EchoKitSessionAdapter {
    /// 创建新的适配器
    pub fn new(
        echokit_client: Arc<EchoKitClient>,
        connection_manager: Arc<DeviceConnectionManager>,
        audio_receiver: mpsc::UnboundedReceiver<(String, Vec<u8>)>,
    ) -> Self {
        Self {
            echokit_client,
            connection_manager,
            session_mapping: Arc::new(RwLock::new(HashMap::new())),
            audio_receiver: Arc::new(RwLock::new(Some(audio_receiver))),
        }
    }

    /// 创建 EchoKit 会话
    pub async fn create_echokit_session(
        &self,
        bridge_session_id: String,
        device_id: String,
        config: EchoKitConfig,
    ) -> Result<String> {
        // 生成 EchoKit 会话 ID
        let echokit_session_id = format!("ek_{}", uuid::Uuid::new_v4());

        info!(
            "Creating EchoKit session: bridge={}, device={}, echokit={}",
            bridge_session_id, device_id, echokit_session_id
        );

        // 调用 EchoKit 客户端启动会话
        self.echokit_client
            .start_session(echokit_session_id.clone(), device_id.clone(), config)
            .await
            .with_context(|| "Failed to start EchoKit session")?;

        // 保存映射关系
        let mut mapping = self.session_mapping.write().await;
        mapping.insert(
            bridge_session_id.clone(),
            (device_id.clone(), echokit_session_id.clone()),
        );

        info!("EchoKit session created successfully: {}", echokit_session_id);
        Ok(echokit_session_id)
    }

    /// 转发音频到 EchoKit
    pub async fn forward_audio(
        &self,
        bridge_session_id: &str,
        audio_data: Vec<u8>,
    ) -> Result<()> {
        // 获取映射信息
        let mapping = self.session_mapping.read().await;
        let (device_id, echokit_session_id) = mapping
            .get(bridge_session_id)
            .ok_or_else(|| anyhow::anyhow!("Session {} not found", bridge_session_id))?
            .clone();
        drop(mapping);

        debug!(
            "Forwarding {} bytes audio from bridge session {} to EchoKit session {}",
            audio_data.len(),
            bridge_session_id,
            echokit_session_id
        );

        // 发送音频到 EchoKit
        self.echokit_client
            .send_audio_data(
                echokit_session_id,
                device_id,
                audio_data,
                AudioFormat::PCM16, // PCM 16-bit format
                false,
            )
            .await
            .with_context(|| "Failed to send audio to EchoKit")?;

        Ok(())
    }

    /// 启动音频接收器（从 EchoKit 接收音频并路由到设备）
    pub async fn start_audio_receiver(self: Arc<Self>) {
        info!("Starting EchoKit audio receiver");

        // 获取音频接收通道
        let mut audio_rx = {
            let mut receiver_guard = self.audio_receiver.write().await;
            receiver_guard.take()
        };

        if audio_rx.is_none() {
            error!("Audio receiver channel not available");
            return;
        }

        let mut audio_rx = audio_rx.unwrap();

        // 持续监听音频数据
        while let Some((echokit_session_id, audio_data)) = audio_rx.recv().await {
            debug!(
                "Received audio from EchoKit session {}: {} bytes",
                echokit_session_id,
                audio_data.len()
            );

            // 根据 echokit_session_id 找到对应的 bridge_session_id
            let bridge_session_id = self.get_bridge_session(&echokit_session_id).await;

            if let Some(bridge_session_id) = bridge_session_id {
                // 将音频数据路由到对应的设备
                match self
                    .connection_manager
                    .push_audio_by_session(&bridge_session_id, audio_data)
                    .await
                {
                    Ok(_) => {
                        debug!("Audio routed to bridge session {}", bridge_session_id);
                    }
                    Err(e) => {
                        error!(
                            "Failed to route audio to bridge session {}: {}",
                            bridge_session_id, e
                        );
                    }
                }
            } else {
                warn!(
                    "No bridge session found for EchoKit session {}",
                    echokit_session_id
                );
            }
        }

        info!("Audio receiver stopped");
    }

    /// 关闭 EchoKit 会话
    pub async fn close_echokit_session(&self, bridge_session_id: &str) -> Result<()> {
        // 获取映射信息
        let mut mapping = self.session_mapping.write().await;
        let (device_id, echokit_session_id) = mapping
            .remove(bridge_session_id)
            .ok_or_else(|| anyhow::anyhow!("Session {} not found", bridge_session_id))?;

        info!(
            "Closing EchoKit session: bridge={}, echokit={}",
            bridge_session_id, echokit_session_id
        );

        // 结束 EchoKit 会话
        self.echokit_client
            .end_session(echokit_session_id, device_id, "session_closed".to_string())
            .await
            .with_context(|| "Failed to end EchoKit session")?;

        Ok(())
    }

    /// 获取 Bridge Session ID（从 EchoKit Session ID）
    pub async fn get_bridge_session(&self, echokit_session_id: &str) -> Option<String> {
        let mapping = self.session_mapping.read().await;

        for (bridge_id, (_, ek_id)) in mapping.iter() {
            if ek_id == echokit_session_id {
                return Some(bridge_id.clone());
            }
        }

        None
    }

    /// 获取设备 ID（从 Bridge Session ID）
    pub async fn get_device_id(&self, bridge_session_id: &str) -> Option<String> {
        let mapping = self.session_mapping.read().await;
        mapping.get(bridge_session_id).map(|(device_id, _)| device_id.clone())
    }

    /// 获取活跃会话数量
    pub async fn get_active_sessions_count(&self) -> usize {
        let mapping = self.session_mapping.read().await;
        mapping.len()
    }

    /// 检查会话是否存在
    pub async fn has_session(&self, bridge_session_id: &str) -> bool {
        let mapping = self.session_mapping.read().await;
        mapping.contains_key(bridge_session_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::websocket::connection_manager::DeviceConnectionManager;

    #[tokio::test]
    async fn test_adapter_creation() {
        let echokit_client = Arc::new(EchoKitClient::new("ws://localhost:9988".to_string()));
        let conn_mgr = Arc::new(DeviceConnectionManager::new());
        let (_tx, rx) = mpsc::unbounded_channel();

        let adapter = EchoKitSessionAdapter::new(echokit_client, conn_mgr, rx);
        assert_eq!(adapter.get_active_sessions_count().await, 0);
    }
}
