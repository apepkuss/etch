use anyhow::{Context, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tracing::{debug, error, info, warn};

use crate::echokit_client::EchoKitClient;
use crate::websocket::connection_manager::DeviceConnectionManager;
use crate::websocket::protocol::ServerEvent;
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
    /// ASR 接收通道
    asr_receiver: Arc<RwLock<Option<mpsc::UnboundedReceiver<(String, String)>>>>,
    /// 原始消息接收通道（用于直接转发 MessagePack 数据）
    raw_message_receiver: Arc<RwLock<Option<mpsc::UnboundedReceiver<(String, Vec<u8>)>>>>,
}

impl EchoKitSessionAdapter {
    /// 创建新的适配器
    pub fn new(
        echokit_client: Arc<EchoKitClient>,
        connection_manager: Arc<DeviceConnectionManager>,
        audio_receiver: mpsc::UnboundedReceiver<(String, Vec<u8>)>,
        asr_receiver: mpsc::UnboundedReceiver<(String, String)>,
        raw_message_receiver: mpsc::UnboundedReceiver<(String, Vec<u8>)>,
    ) -> Self {
        Self {
            echokit_client,
            connection_manager,
            session_mapping: Arc::new(RwLock::new(HashMap::new())),
            audio_receiver: Arc::new(RwLock::new(Some(audio_receiver))),
            asr_receiver: Arc::new(RwLock::new(Some(asr_receiver))),
            raw_message_receiver: Arc::new(RwLock::new(Some(raw_message_receiver))),
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

        // 🔑 关键修复：在调用 start_session 之前，立即在 active_sessions 中预注册
        // 这样可以确保当 EchoKit Server 返回 HelloChunk 时，转发循环能找到 session
        self.echokit_client
            .pre_register_session(echokit_session_id.clone(), device_id.clone())
            .await;

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

        // 发送音频到 EchoKit（StartChat已在会话创建时发送）
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

    /// 提交音频进行处理（发送Submit消息到EchoKit）
    pub async fn submit_audio_for_processing(&self, bridge_session_id: &str) -> Result<()> {
        // 获取映射信息
        let mapping = self.session_mapping.read().await;
        let (device_id, echokit_session_id) = mapping
            .get(bridge_session_id)
            .ok_or_else(|| anyhow::anyhow!("Session {} not found", bridge_session_id))?
            .clone();
        drop(mapping);

        info!(
            "📤 Submitting audio for processing: bridge={}, echokit={}",
            bridge_session_id, echokit_session_id
        );

        // 发送Submit命令到EchoKit
        self.echokit_client
            .send_submit_command()
            .await
            .with_context(|| "Failed to send submit command to EchoKit")?;

        info!("✅ Submit command sent successfully to EchoKit");
        Ok(())
    }

    /// 发送StartChat命令到EchoKit（开始新的对话会话）
    pub async fn send_start_chat(&self, echokit_session_id: &str) -> Result<()> {
        info!("📤 Sending StartChat command to EchoKit for session {}", echokit_session_id);

        self.echokit_client
            .send_start_chat_command()
            .await
            .with_context(|| "Failed to send StartChat command to EchoKit")?;

        info!("✅ StartChat command sent successfully to EchoKit for session {}", echokit_session_id);

        // 🎁 发送完 StartChat 后，立即发送缓存的 Hello 消息
        info!("🎁 Triggering cached Hello messages for session {}", echokit_session_id);
        self.echokit_client.check_and_send_cached_hello(echokit_session_id).await;

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

            // 根据 echokit_session_id 找到对应的 bridge_session_id 和 device_id
            let session_info = {
                let mapping = self.session_mapping.read().await;
                mapping
                    .iter()
                    .find(|(_, (_, ek_id))| ek_id == &echokit_session_id)
                    .map(|(bridge_id, (dev_id, _))| (bridge_id.clone(), dev_id.clone()))
            };

            if let Some((bridge_session_id, device_id)) = session_info {
                // 发送 StartAudio 事件
                if let Err(e) = self
                    .connection_manager
                    .send_server_event(
                        &device_id,
                        ServerEvent::StartAudio {
                            text: "语音回复".to_string(),
                        },
                    )
                    .await
                {
                    error!(
                        "Failed to send StartAudio event to device {}: {}",
                        device_id, e
                    );
                    continue;
                }

                debug!(
                    "Sent StartAudio event to device {} for bridge session {}",
                    device_id, bridge_session_id
                );

                // 分块发送音频数据（每块 2048 字节）
                const CHUNK_SIZE: usize = 2048;
                let chunks: Vec<_> = audio_data.chunks(CHUNK_SIZE).collect();
                let total_chunks = chunks.len();

                for (index, chunk) in chunks.into_iter().enumerate() {
                    match self
                        .connection_manager
                        .send_server_event(
                            &device_id,
                            ServerEvent::AudioChunk {
                                data: chunk.to_vec(),
                            },
                        )
                        .await
                    {
                        Ok(_) => {
                            if (index + 1) % 10 == 0 || index + 1 == total_chunks {
                                debug!(
                                    "Sent audio chunk {}/{} ({} bytes) to device {}",
                                    index + 1,
                                    total_chunks,
                                    chunk.len(),
                                    device_id
                                );
                            }
                        }
                        Err(e) => {
                            error!(
                                "Failed to send audio chunk {}/{} to device {}: {}",
                                index + 1,
                                total_chunks,
                                device_id,
                                e
                            );
                            break;
                        }
                    }
                }

                // 发送 EndAudio 事件
                if let Err(e) = self
                    .connection_manager
                    .send_server_event(&device_id, ServerEvent::EndAudio)
                    .await
                {
                    error!(
                        "Failed to send EndAudio event to device {}: {}",
                        device_id, e
                    );
                } else {
                    info!(
                        "Completed audio stream to device {}: {} chunks ({} bytes total)",
                        device_id,
                        total_chunks,
                        audio_data.len()
                    );
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

    /// 启动 ASR 接收器（从 EchoKit 接收 ASR 结果并路由到设备）
    pub async fn start_asr_receiver(self: Arc<Self>) {
        info!("🎙️ Starting EchoKit ASR receiver");

        // 获取 ASR 接收通道
        let mut asr_rx = {
            let mut receiver_guard = self.asr_receiver.write().await;
            receiver_guard.take()
        };

        if asr_rx.is_none() {
            error!("❌ ASR receiver channel not available");
            return;
        }

        let mut asr_rx = asr_rx.unwrap();
        info!("✅ ASR receiver channel acquired, waiting for messages...");

        // 持续监听 ASR 数据
        while let Some((echokit_session_id, asr_text)) = asr_rx.recv().await {
            info!(
                "📝 Received ASR from EchoKit session {}: {}",
                echokit_session_id, asr_text
            );

            // 根据 echokit_session_id 找到对应的 device_id
            let device_id = {
                let mapping = self.session_mapping.read().await;
                let device_id = mapping
                    .iter()
                    .find(|(_, (_, ek_id))| ek_id == &echokit_session_id)
                    .map(|(_, (dev_id, _))| dev_id.clone());

                if device_id.is_none() {
                    warn!("⚠️ No device found for EchoKit session {} in mapping", echokit_session_id);
                    debug!("Current session mapping: {:?}", *mapping);
                }
                device_id
            };

            if let Some(device_id) = device_id {
                info!("🎯 Found device {} for ASR, forwarding...", device_id);

                // 发送 ASR 事件到设备
                match self
                    .connection_manager
                    .send_server_event(
                        &device_id,
                        ServerEvent::ASR {
                            text: asr_text.clone(),
                        },
                    )
                    .await
                {
                    Ok(_) => {
                        info!(
                            "✅ Successfully forwarded ASR to device {}: {}",
                            device_id, asr_text
                        );
                    }
                    Err(e) => {
                        error!(
                            "❌ Failed to forward ASR to device {}: {}",
                            device_id, e
                        );
                    }
                }
            } else {
                warn!(
                    "⚠️ No device found for EchoKit session {} (ASR: {})",
                    echokit_session_id, asr_text
                );
            }
        }

        info!("ASR receiver stopped");
    }

    /// 启动原始消息接收器（直接转发 MessagePack 数据到设备）
    pub async fn start_raw_message_receiver(self: Arc<Self>) {
        info!("📦 Starting EchoKit raw message receiver");

        // 获取原始消息接收通道
        let mut raw_msg_rx = {
            let mut receiver_guard = self.raw_message_receiver.write().await;
            receiver_guard.take()
        };

        if raw_msg_rx.is_none() {
            error!("❌ Raw message receiver channel not available");
            return;
        }

        let mut raw_msg_rx = raw_msg_rx.unwrap();
        info!("✅ Raw message receiver channel acquired, waiting for messages...");

        // 持续监听原始消息数据
        while let Some((echokit_session_id, raw_data)) = raw_msg_rx.recv().await {
            debug!(
                "📦 Received raw message from EchoKit session {}: {} bytes",
                echokit_session_id,
                raw_data.len()
            );

            // 根据 echokit_session_id 找到对应的 device_id
            let device_id = {
                let mapping = self.session_mapping.read().await;
                mapping
                    .iter()
                    .find(|(_, (_, ek_id))| ek_id == &echokit_session_id)
                    .map(|(_, (dev_id, _))| dev_id.clone())
            };

            if let Some(device_id) = device_id {
                // 直接发送原始二进制数据到设备
                match self.connection_manager.send_binary(&device_id, raw_data).await {
                    Ok(_) => {
                        debug!(
                            "✅ Successfully forwarded raw message to device {}",
                            device_id
                        );
                    }
                    Err(e) => {
                        error!(
                            "❌ Failed to forward raw message to device {}: {}",
                            device_id, e
                        );
                    }
                }
            } else {
                warn!(
                    "⚠️ No device found for EchoKit session {} (raw message)",
                    echokit_session_id
                );
            }
        }

        info!("Raw message receiver stopped");
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
        let echokit_client = Arc::new(EchoKitClient::new("wss://indie.echokit.dev/ws/test-visitor".to_string()));
        let conn_mgr = Arc::new(DeviceConnectionManager::new());
        let (_tx, rx) = mpsc::unbounded_channel();
        let (_asr_tx, asr_rx) = mpsc::unbounded_channel();

        let adapter = EchoKitSessionAdapter::new(echokit_client, conn_mgr, rx, asr_rx);
        assert_eq!(adapter.get_active_sessions_count().await, 0);
    }
}
