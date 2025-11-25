use anyhow::{Context, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tracing::{debug, error, info, warn};

use crate::echokit_client::EchoKitClient;
use crate::websocket::connection_manager::DeviceConnectionManager;
use crate::websocket::session_manager::SessionManager;
use crate::websocket::protocol::ServerEvent;
use echo_shared::{AudioFormat, EchoKitConfig};

/// EchoKit 会话适配器 - 负责 Bridge Session 和 EchoKit 的集成
pub struct EchoKitSessionAdapter {
    /// EchoKit 客户端
    echokit_client: Arc<EchoKitClient>,
    /// 设备连接管理器（用于发送音频到设备）
    connection_manager: Arc<DeviceConnectionManager>,
    /// 🔧 会话管理器（用于保存 ASR 转录文本到内存）
    session_manager: Arc<SessionManager>,
    /// Session 映射: bridge_session_id -> (device_id, echokit_session_id)
    session_mapping: Arc<RwLock<HashMap<String, (String, String)>>>,
    /// 音频接收通道
    audio_receiver: Arc<RwLock<Option<mpsc::UnboundedReceiver<(String, Vec<u8>)>>>>,
    /// ASR 接收通道
    asr_receiver: Arc<RwLock<Option<mpsc::UnboundedReceiver<(String, String)>>>>,
    /// AI 回复接收通道
    response_receiver: Arc<RwLock<Option<mpsc::UnboundedReceiver<(String, String)>>>>,
    /// 原始消息接收通道（用于直接转发 MessagePack 数据）
    raw_message_receiver: Arc<RwLock<Option<mpsc::UnboundedReceiver<(String, Vec<u8>)>>>>,
}

impl EchoKitSessionAdapter {
    /// 创建新的适配器
    pub fn new(
        echokit_client: Arc<EchoKitClient>,
        connection_manager: Arc<DeviceConnectionManager>,
        session_manager: Arc<SessionManager>,
        audio_receiver: mpsc::UnboundedReceiver<(String, Vec<u8>)>,
        asr_receiver: mpsc::UnboundedReceiver<(String, String)>,
        response_receiver: mpsc::UnboundedReceiver<(String, String)>,
        raw_message_receiver: mpsc::UnboundedReceiver<(String, Vec<u8>)>,
    ) -> Self {
        Self {
            echokit_client,
            connection_manager,
            session_manager,
            session_mapping: Arc::new(RwLock::new(HashMap::new())),
            audio_receiver: Arc::new(RwLock::new(Some(audio_receiver))),
            asr_receiver: Arc::new(RwLock::new(Some(asr_receiver))),
            response_receiver: Arc::new(RwLock::new(Some(response_receiver))),
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
        let start_time = std::time::Instant::now();

        // 生成 EchoKit 会话 ID
        let echokit_session_id = format!("ek_{}", uuid::Uuid::new_v4());

        info!(
            "Creating EchoKit session: bridge={}, device={}, echokit={}",
            bridge_session_id, device_id, echokit_session_id
        );

        // 🔧 新增：确保 EchoKit 连接使用正确的 device_id
        // 如果尚未连接或需要重新连接到不同的 device_id，则重新连接
        if !self.echokit_client.is_connected().await {
            info!("EchoKit not connected, connecting with device_id: {}", device_id);
            self.echokit_client
                .connect_with_device_id(Some(&device_id))
                .await
                .with_context(|| format!("Failed to connect to EchoKit with device_id: {}", device_id))?;
        }

        // 🔑 关键修复：在调用 start_session 之前，立即在 active_sessions 中预注册
        // 这样可以确保当 EchoKit Server 返回 HelloChunk 时，转发循环能找到 session
        self.echokit_client
            .pre_register_session(echokit_session_id.clone(), device_id.clone())
            .await;

        let pre_register_elapsed = start_time.elapsed();
        info!("⏱️ Pre-registration took: {:.3}s", pre_register_elapsed.as_secs_f64());

        // 调用 EchoKit 客户端启动会话
        let session_start_time = std::time::Instant::now();
        self.echokit_client
            .start_session(echokit_session_id.clone(), device_id.clone(), config)
            .await
            .with_context(|| "Failed to start EchoKit session")?;

        let session_start_elapsed = session_start_time.elapsed();
        info!("⏱️ start_session took: {:.3}s", session_start_elapsed.as_secs_f64());

        // 保存映射关系
        let mut mapping = self.session_mapping.write().await;
        mapping.insert(
            bridge_session_id.clone(),
            (device_id.clone(), echokit_session_id.clone()),
        );

        let total_elapsed = start_time.elapsed();
        info!(
            "⏱️ EchoKit session created successfully: {} (total time: {:.3}s)",
            echokit_session_id,
            total_elapsed.as_secs_f64()
        );

        if total_elapsed.as_secs() > 5 {
            warn!(
                "⚠️ EchoKit Session creation took unusually long: {:.3}s (expected < 5s)",
                total_elapsed.as_secs_f64()
            );
        }

        Ok(echokit_session_id)
    }

    /// 注册 Bridge 会话到现有的 EchoKit 会话（复用 EchoKit 会话）
    pub async fn register_bridge_session(
        &self,
        bridge_session_id: String,
        device_id: String,
        echokit_session_id: String,
    ) -> Result<()> {
        info!(
            "Registering bridge session {} to existing EchoKit session {} for device {}",
            bridge_session_id, echokit_session_id, device_id
        );

        // 保存映射关系
        let mut mapping = self.session_mapping.write().await;
        mapping.insert(
            bridge_session_id.clone(),
            (device_id.clone(), echokit_session_id.clone()),
        );
        drop(mapping);

        // 🔑 重新注册 EchoKit Session ID 到 active_sessions
        // 确保 ASR 等消息可以正确转发
        self.echokit_client
            .pre_register_session(echokit_session_id.clone(), device_id.clone())
            .await;

        // 🎁 修复：复用会话时也要发送缓存的 Hello 消息给新客户端
        // 虽然 EchoKit 会话被复用，但对于新的 Bridge 客户端来说，
        // 这是首次连接，用户期望看到问候语
        info!("🎁 Triggering cached Hello messages for reused session {}", echokit_session_id);
        self.echokit_client.check_and_send_cached_hello(&echokit_session_id).await;

        info!(
            "✅ Bridge session {} registered successfully to EchoKit session {}",
            bridge_session_id, echokit_session_id
        );
        Ok(())
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

    /// 根据 Bridge Session ID 发送 StartChat 命令
    /// 这个方法会查找对应的 EchoKit Session 并发送 StartChat
    pub async fn send_start_chat_for_session(&self, bridge_session_id: &str) -> Result<()> {
        // 首先获取 EchoKit session ID（作用域结束后自动释放锁）
        let echokit_session_id = {
            let session_mapping = self.session_mapping.read().await;

            if let Some((_, echokit_session_id)) = session_mapping.get(bridge_session_id) {
                echokit_session_id.clone()
            } else {
                anyhow::bail!("Bridge session {} not found in session mapping", bridge_session_id);
            }
        }; // session_mapping 锁在此释放

        debug!(
            "Sending StartChat for bridge session {} -> EchoKit session {}",
            bridge_session_id, echokit_session_id
        );

        // 调用原有的 send_start_chat 方法
        self.send_start_chat(&echokit_session_id).await
    }

    /// 启动音频接收器（从 EchoKit 接收原始 MessagePack 数据并直接转发到设备）
    ///
    /// 修复说明：移除了音频解包、过滤和重新封装的逻辑，改为直接转发原始 MessagePack 数据。
    /// 这样可以：
    /// 1. 避免丢失小音频片段（之前 < 100 字节的会被过滤）
    /// 2. 保持数据格式与 EchoKit Server 完全一致
    /// 3. 让客户端 WebUI 自己解析和处理数据
    pub async fn start_audio_receiver(self: Arc<Self>) {
        info!("🎧 Starting EchoKit MessagePack data receiver (direct forwarding mode)");

        // 获取音频接收通道
        let mut audio_rx = {
            let mut receiver_guard = self.audio_receiver.write().await;
            receiver_guard.take()
        };

        if audio_rx.is_none() {
            error!("❌ Audio receiver channel not available");
            return;
        }

        let mut audio_rx = audio_rx.unwrap();
        info!("✅ Audio receiver channel acquired, waiting for MessagePack data...");

        // 持续监听 MessagePack 数据
        while let Some((echokit_session_id, raw_messagepack_data)) = audio_rx.recv().await {
            debug!(
                "📦 Received MessagePack data from EchoKit session {}: {} bytes",
                echokit_session_id,
                raw_messagepack_data.len()
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
                // 直接转发原始 MessagePack 数据到设备，不做任何处理
                match self.connection_manager.send_binary(&device_id, raw_messagepack_data.clone()).await {
                    Ok(_) => {
                        debug!(
                            "✅ Successfully forwarded {} bytes MessagePack data to device {}",
                            raw_messagepack_data.len(),
                            device_id
                        );
                    }
                    Err(e) => {
                        error!(
                            "❌ Failed to forward MessagePack data to device {}: {}",
                            device_id, e
                        );
                    }
                }
            } else {
                warn!(
                    "⚠️ No device found for EchoKit session {} (MessagePack data)",
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

                // 🔧 方案B：先保存 ASR 文本到内存（找到对应的 bridge_session_id）
                let bridge_session_id = {
                    let mapping = self.session_mapping.read().await;
                    mapping
                        .iter()
                        .find(|(_, (_, ek_id))| ek_id == &echokit_session_id)
                        .map(|(bridge_id, _)| bridge_id.clone())
                };

                if let Some(bridge_session_id) = bridge_session_id {
                    // 将 ASR 文本追加到会话的转录记录中
                    self.session_manager.append_transcript(&bridge_session_id, asr_text.clone()).await;
                    info!("💾 Saved ASR text to session {} memory", bridge_session_id);
                } else {
                    warn!("⚠️ Could not find bridge session for EchoKit session {}", echokit_session_id);
                }

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

    /// 启动 AI 回复接收器（从 EchoKit 接收 AI 回复文本并保存到 SessionManager）
    pub async fn start_response_receiver(self: Arc<Self>) {
        info!("🤖 Starting EchoKit AI response receiver");

        // 获取 AI 回复接收通道
        let mut response_rx = {
            let mut receiver_guard = self.response_receiver.write().await;
            receiver_guard.take()
        };

        if response_rx.is_none() {
            error!("❌ AI response receiver channel not available");
            return;
        }

        let mut response_rx = response_rx.unwrap();
        info!("✅ AI response receiver channel acquired, waiting for messages...");

        // 持续监听 AI 回复数据
        while let Some((echokit_session_id, response_text)) = response_rx.recv().await {
            info!(
                "🤖 Received AI response from EchoKit session {}: {}",
                echokit_session_id, response_text
            );

            // 根据 echokit_session_id 找到对应的 bridge_session_id
            let bridge_session_id = {
                let mapping = self.session_mapping.read().await;
                mapping
                    .iter()
                    .find(|(_, (_, ek_id))| ek_id == &echokit_session_id)
                    .map(|(bridge_id, _)| bridge_id.clone())
            };

            if let Some(bridge_session_id) = bridge_session_id {
                // 🔧 检测 EndResponse 特殊标记
                if response_text == "__END_RESPONSE__" {
                    // 收到 EndResponse 事件，合并当前轮次的 AI 回复
                    info!("🔔 Received EndResponse signal for session {}, finalizing current round response", bridge_session_id);
                    self.session_manager.finalize_current_round_response(&bridge_session_id).await;
                } else {
                    // 正常的 AI 回复片段，追加到当前轮次的回复记录中
                    self.session_manager.append_response(&bridge_session_id, response_text.clone()).await;
                    info!("💾 Saved AI response fragment to session {} memory", bridge_session_id);
                }
            } else {
                warn!("⚠️ Could not find bridge session for EchoKit session {} (AI response)", echokit_session_id);
            }
        }

        info!("AI response receiver stopped");
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
