use anyhow::{Context, Result};
use echo_shared::{
    EchoKitClientMessage, EchoKitServerMessage, EchoKitConfig, EchoKitServiceStatus,
    WebSocketMessage, AudioFormat
};
use chrono::Utc;
use futures::{SinkExt, StreamExt};
use serde_json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use tracing::{info, warn, error, debug};
use url::Url;

// EchoKit WebSocket 客户端
#[derive(Clone)]
pub struct EchoKitClient {
    websocket_url: String,
    ws_stream: Arc<RwLock<Option<WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>>>>,
    is_connected: Arc<RwLock<bool>>,
    service_status: Arc<RwLock<Option<EchoKitServiceStatus>>>,
    message_sender: mpsc::UnboundedSender<EchoKitClientMessage>,
    message_receiver: Arc<RwLock<Option<mpsc::UnboundedReceiver<EchoKitClientMessage>>>>,
    active_sessions: Arc<RwLock<HashMap<String, String>>>, // session_id -> device_id
}

impl EchoKitClient {
    pub fn new(websocket_url: String) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();

        Self {
            websocket_url,
            ws_stream: Arc::new(RwLock::new(None)),
            is_connected: Arc::new(RwLock::new(false)),
            service_status: Arc::new(RwLock::new(None)),
            message_sender: tx,
            message_receiver: Arc::new(RwLock::new(Some(rx))),
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // 连接到 EchoKit Server
    pub async fn connect(&self) -> Result<()> {
        let url = Url::parse(&self.websocket_url)
            .with_context(|| format!("Invalid WebSocket URL: {}", self.websocket_url))?;

        info!("Connecting to EchoKit Server at: {}", url);

        match connect_async(url).await {
            Ok((ws_stream, response)) => {
                info!("Connected to EchoKit Server successfully");
                debug!("Response status: {}", response.status());

                *self.ws_stream.write().await = Some(ws_stream);
                *self.is_connected.write().await = true;

                // 发送服务就绪消息
                if let Err(e) = self.send_service_ready().await {
                    warn!("Failed to send service ready message: {}", e);
                }

                // 启动消息处理任务
                self.start_message_handler().await?;

                Ok(())
            }
            Err(e) => {
                error!("Failed to connect to EchoKit Server: {}", e);
                Err(anyhow::anyhow!("Connection failed: {}", e))
            }
        }
    }

    // 断开连接
    pub async fn disconnect(&self) -> Result<()> {
        info!("Disconnecting from EchoKit Server");

        *self.is_connected.write().await = false;

        if let Some(mut ws_stream) = self.ws_stream.write().await.take() {
            let _ = ws_stream.close(None).await;
        }

        Ok(())
    }

    // 检查连接状态
    pub async fn is_connected(&self) -> bool {
        *self.is_connected.read().await
    }

    // 获取服务状态
    pub async fn get_service_status(&self) -> Option<EchoKitServiceStatus> {
        self.service_status.read().await.clone()
    }

    // 发送消息到 EchoKit Server
    pub async fn send_message(&self, message: EchoKitClientMessage) -> Result<()> {
        if !self.is_connected().await {
            return Err(anyhow::anyhow!("Not connected to EchoKit Server"));
        }

        // 记录会话信息
        if let EchoKitClientMessage::StartSession { session_id, device_id, .. } = &message {
            self.active_sessions.write().await.insert(session_id.clone(), device_id.clone());
        }

        // 实现WebSocket消息发送
        let json_message = serde_json::to_string(&message)
            .with_context(|| "Failed to serialize message")?;

        debug!("Sending message to EchoKit Server: {}", json_message);

        // 获取WebSocket流并发送消息
        let mut ws_stream_guard = self.ws_stream.write().await;
        if let Some(ws_stream) = ws_stream_guard.as_mut() {
            if let Err(e) = ws_stream.send(Message::Text(json_message)).await {
                error!("Failed to send message to EchoKit Server: {}", e);
                *self.is_connected.write().await = false;
                return Err(anyhow::anyhow!("WebSocket send error: {}", e));
            }
            debug!("Message sent to EchoKit Server successfully");
        } else {
            return Err(anyhow::anyhow!("WebSocket stream not available"));
        }

        Ok(())
    }

    // 发送初始session.update消息 (作为服务就绪信号)
    pub async fn send_service_ready(&self) -> Result<()> {
        info!("Sending initial session update to EchoKit Server as service ready signal");
        self.send_session_update().await
    }

    // 开始会话
    pub async fn start_session(
        &self,
        session_id: String,
        device_id: String,
        config: EchoKitConfig,
    ) -> Result<()> {
        let message = EchoKitClientMessage::StartSession {
            session_id,
            device_id,
            config,
        };

        self.send_message(message).await
    }

    // 结束会话
    pub async fn end_session(
        &self,
        session_id: String,
        device_id: String,
        reason: String,
    ) -> Result<()> {
        // 从活跃会话中移除
        self.active_sessions.write().await.remove(&session_id);

        let message = EchoKitClientMessage::EndSession {
            session_id,
            device_id,
            reason,
        };

        self.send_message(message).await
    }

    // 发送音频数据
    pub async fn send_audio_data(
        &self,
        session_id: String,
        device_id: String,
        audio_data: Vec<u8>,
        format: AudioFormat,
        is_final: bool,
    ) -> Result<()> {
        let message = EchoKitClientMessage::AudioData {
            session_id,
            device_id,
            audio_data,
            format,
            is_final,
        };

        self.send_message(message).await
    }

    // 发送 Ping
    pub async fn ping(&self) -> Result<()> {
        self.send_message(EchoKitClientMessage::Ping).await
    }

    // 发送 OpenAI 格式的 session.update 事件来保持连接
    pub async fn send_session_update(&self) -> Result<()> {
        use echo_shared::{OpenAIClientEvent, OpenAISessionConfig};

        let session_update = OpenAIClientEvent::SessionUpdate {
            event_id: Some(format!("evt_{}", uuid::Uuid::new_v4())),
            session: OpenAISessionConfig {
                instructions: Some("Bridge client connected".to_string()),
                voice: Some("speaker2".to_string()),
                temperature: Some(0.8),
            },
        };

        let json_message = serde_json::to_string(&session_update)
            .with_context(|| "Failed to serialize session update")?;

        if !self.is_connected().await {
            return Err(anyhow::anyhow!("Not connected to EchoKit Server"));
        }

        debug!("Sending OpenAI session update: {}", json_message);

        // 获取WebSocket流并发送消息
        let mut ws_stream_guard = self.ws_stream.write().await;
        if let Some(ws_stream) = ws_stream_guard.as_mut() {
            if let Err(e) = ws_stream.send(Message::Text(json_message)).await {
                error!("Failed to send session update: {}", e);
                *self.is_connected.write().await = false;
                return Err(anyhow::anyhow!("WebSocket send error: {}", e));
            }
            info!("OpenAI session update sent successfully");
        } else {
            return Err(anyhow::anyhow!("WebSocket stream not available"));
        }

        Ok(())
    }

    // 启动消息处理任务
    async fn start_message_handler(&self) -> Result<()> {
        let ws_stream = self.ws_stream.clone();
        let is_connected = self.is_connected.clone();
        let service_status = self.service_status.clone();
        let active_sessions = self.active_sessions.clone();

        // 为每个连接创建独立的消息通道
        let (tx, mut rx) = mpsc::unbounded_channel::<EchoKitClientMessage>();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    // 处理来自 EchoKit Server 的消息
                    message_result = async {
                        let mut ws_stream_guard = ws_stream.write().await;
                        if let Some(ws_stream) = ws_stream_guard.as_mut() {
                            ws_stream.next().await
                        } else {
                            None
                        }
                    } => {
                        match message_result {
                            Some(Ok(Message::Text(text))) => {
                                debug!("Received text message from EchoKit Server: {}", text);
                                if let Err(e) = Self::handle_server_message(
                                    text,
                                    &service_status,
                                    &active_sessions,
                                ).await {
                                    error!("Error handling server message: {}", e);
                                }
                            }
                            Some(Ok(Message::Binary(data))) => {
                                debug!("Received binary data from EchoKit Server: {} bytes", data.len());
                                // 处理二进制音频数据
                                if let Err(e) = Self::handle_binary_audio_data(
                                    data,
                                    &service_status,
                                    &active_sessions,
                                ).await {
                                    error!("Error handling binary audio data: {}", e);
                                }
                            }
                            Some(Ok(Message::Close(close_frame))) => {
                                info!("EchoKit Server closed connection: {:?}", close_frame);
                                *is_connected.write().await = false;
                                break;
                            }
                            Some(Ok(Message::Ping(payload))) => {
                                debug!("Received ping from EchoKit Server");
                                // 自动回复pong
                                let mut ws_stream_guard = ws_stream.write().await;
                                if let Some(ws_stream) = ws_stream_guard.as_mut() {
                                    if let Err(e) = ws_stream.send(Message::Pong(payload)).await {
                                        error!("Failed to send pong: {}", e);
                                        *is_connected.write().await = false;
                                        break;
                                    }
                                }
                            }
                            Some(Ok(Message::Pong(_))) => {
                                debug!("Received pong from EchoKit Server");
                            }
                            Some(Ok(Message::Frame(_))) => {
                                debug!("Received WebSocket frame from EchoKit Server");
                                // WebSocket frames are handled internally by tungstenite
                            }
                            Some(Err(e)) => {
                                error!("WebSocket error from EchoKit Server: {}", e);
                                *is_connected.write().await = false;
                                break;
                            }
                            None => {
                                warn!("WebSocket stream ended");
                                *is_connected.write().await = false;
                                break;
                            }
                        }
                    }

                    
                    // 定期心跳
                    _ = tokio::time::sleep(tokio::time::Duration::from_secs(30)) => {
                        debug!("Sending heartbeat to EchoKit Server");
                        let mut ws_stream_guard = ws_stream.write().await;
                        if let Some(ws_stream) = ws_stream_guard.as_mut() {
                            if let Err(e) = ws_stream.send(Message::Ping(vec![])).await {
                                error!("Failed to send ping to EchoKit Server: {}", e);
                                *is_connected.write().await = false;
                                break;
                            }
                            debug!("Heartbeat sent successfully");
                        } else {
                            warn!("WebSocket not available for heartbeat");
                        }
                    }
                }
            }
        });

        Ok(())
    }

    // 处理来自 EchoKit Server 的消息
    async fn handle_server_message(
        text: String,
        service_status: &Arc<RwLock<Option<EchoKitServiceStatus>>>,
        active_sessions: &Arc<RwLock<HashMap<String, String>>>,
    ) -> Result<()> {
        let server_message: EchoKitServerMessage = serde_json::from_str(&text)
            .with_context(|| format!("Failed to parse server message: {}", text))?;

        match server_message {
            // OpenAI realtime API 格式消息处理
            EchoKitServerMessage::SessionCreated { event_id, session } => {
                info!("OpenAI session created: {} (event_id: {})", session.id, event_id);
                info!("Session details: model={}, modalities={:?}", session.model, session.modalities);
                // 存储session ID 映射到设备ID（这里暂时用session.id作为key）
                active_sessions.write().await.insert(session.id.clone(), "bridge_device".to_string());
            }
            EchoKitServerMessage::ConversationCreated { event_id, conversation } => {
                info!("OpenAI conversation created: {} (event_id: {})", conversation.id, event_id);
            }
            EchoKitServerMessage::ResponseText { event_id, session_id, text } => {
                info!("OpenAI text response for session {}: {} (event_id: {})", session_id, text, event_id);
                // 这里可以转发文本响应到设备或其他服务
            }
            EchoKitServerMessage::ResponseAudio { event_id, session_id, audio } => {
                info!("OpenAI audio response for session {} (event_id: {}, audio_len: {})",
                      session_id, event_id, audio.len());
                // 这里可以处理Base64编码的音频数据
            }
            EchoKitServerMessage::OpenAIError { event_id, error } => {
                error!("OpenAI error (event_id: {}): {} - {}", event_id, error.type_, error.message);
            }

            // 原有格式消息处理（向后兼容）
            EchoKitServerMessage::SessionStarted { session_id, device_id, timestamp } => {
                info!("Session started: {} for device: {} at {}", session_id, device_id, timestamp);
                active_sessions.write().await.insert(session_id.clone(), device_id);
            }
            EchoKitServerMessage::SessionEnded { session_id, device_id, reason, timestamp } => {
                info!("Session ended: {} for device: {} (reason: {}) at {}", session_id, device_id, reason, timestamp);
                active_sessions.write().await.remove(&session_id);
            }
            EchoKitServerMessage::Transcription {
                session_id,
                device_id,
                text,
                confidence,
                is_final,
                timestamp
            } => {
                info!("Transcription for session {}: {} (confidence: {:.2}, final: {})",
                      session_id, text, confidence, is_final);
                // 这里可以转发转录结果到前端或其他服务
            }
            EchoKitServerMessage::Response {
                session_id,
                device_id,
                text,
                audio_data,
                is_complete,
                timestamp
            } => {
                info!("Response for session {}: {} (complete: {})", session_id, text, is_complete);
                if let Some(audio) = audio_data {
                    debug!("Received audio data: {} bytes", audio.len());
                }
                // 这里可以转发响应到设备
            }
            EchoKitServerMessage::Error { session_id, device_id, error } => {
                error!("Error for session {}: {} - {}", session_id, error.code, error.message);
                // 这里可以处理错误并通知相关服务
            }
            EchoKitServerMessage::Pong => {
                debug!("Received pong from EchoKit Server");
            }
            EchoKitServerMessage::ServiceStatus { status } => {
                info!("Received service status update: {} active sessions", status.active_sessions);
                *service_status.write().await = Some(status);
            }
        }

        Ok(())
    }

    // 获取活跃会话数量
    pub async fn get_active_sessions_count(&self) -> usize {
        self.active_sessions.read().await.len()
    }

    // 获取所有活跃会话
    pub async fn get_active_sessions(&self) -> HashMap<String, String> {
        self.active_sessions.read().await.clone()
    }
}

// EchoKit 连接管理器
pub struct EchoKitConnectionManager {
    client: Arc<EchoKitClient>,
    reconnect_interval: tokio::time::Duration,
    max_reconnect_attempts: u32,
}

impl EchoKitConnectionManager {
    pub fn new(websocket_url: String) -> Self {
        Self {
            client: Arc::new(EchoKitClient::new(websocket_url)),
            reconnect_interval: tokio::time::Duration::from_secs(5),
            max_reconnect_attempts: 10,
        }
    }

    // 启动连接管理器
    pub async fn start(&self) -> Result<()> {
        let client = self.client.clone();
        let reconnect_interval = self.reconnect_interval;
        let max_reconnect_attempts = self.max_reconnect_attempts;

        tokio::spawn(async move {
            let mut reconnect_attempts = 0;

            loop {
                match client.connect().await {
                    Ok(_) => {
                        info!("EchoKit connection established successfully");
                        reconnect_attempts = 0; // 重置重连计数

                        // 等待连接断开
                        while client.is_connected().await {
                            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                        }

                        warn!("EchoKit connection lost");
                    }
                    Err(e) => {
                        error!("Failed to connect to EchoKit: {}", e);
                    }
                }

                // 如果连接断开，尝试重连
                if reconnect_attempts < max_reconnect_attempts {
                    reconnect_attempts += 1;
                    info!("Attempting to reconnect to EchoKit (attempt {}/{})",
                          reconnect_attempts, max_reconnect_attempts);
                    tokio::time::sleep(reconnect_interval).await;
                } else {
                    error!("Max reconnect attempts reached. Giving up.");
                    break;
                }
            }
        });

        Ok(())
    }

    // 获取客户端实例
    pub fn get_client(&self) -> Arc<EchoKitClient> {
        self.client.clone()
    }
}

impl EchoKitClient {
    // 处理二进制音频数据
    async fn handle_binary_audio_data(
        data: Vec<u8>,
        _service_status: &Arc<RwLock<Option<EchoKitServiceStatus>>>,
        _active_sessions: &Arc<RwLock<HashMap<String, String>>>,
    ) -> Result<()> {
        debug!("Processing binary audio data: {} bytes", data.len());

        // 尝试解析音频数据格式
        if data.len() < 4 {
            warn!("Audio data too small to determine format: {} bytes", data.len());
            return Ok(());
        }

        // 简单的音频格式检测 (前4个字节)
        let format_indicator = &data[0..4];
        let audio_format = match format_indicator {
            b"RIFF" => "WAV",
            b"OggS" => "OGG",
            [0xFF, 0xFB, ..] | [0xFF, 0xF3, ..] | [0xFF, 0xF2, ..] => "MP3",
            _ => {
                // 假设是原始PCM数据
                "PCM16"
            }
        };

        debug!("Detected audio format: {}", audio_format);

        // TODO: 实际处理音频数据，例如：
        // 1. 解码音频格式
        // 2. 转换为需要的格式
        // 3. 发送到相应的设备或处理流程

        info!("Audio data processed successfully (format: {}, size: {} bytes)",
              audio_format, data.len());

        Ok(())
    }
}
