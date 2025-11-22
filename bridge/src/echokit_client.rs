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
    audio_callback: Option<mpsc::UnboundedSender<(String, Vec<u8>)>>, // (session_id, audio_data)
    asr_callback: Option<mpsc::UnboundedSender<(String, String)>>, // (session_id, asr_text)
    raw_message_callback: Option<mpsc::UnboundedSender<(String, Vec<u8>)>>, // (session_id, raw_messagepack_data)
    cached_hello_messages: Arc<RwLock<Vec<Vec<u8>>>>, // 缓存 HelloChunk 消息，用于新会话
    pending_hello_sessions: Arc<RwLock<Vec<String>>>, // 等待发送缓存 Hello 的会话列表
    hello_caching_enabled: Arc<RwLock<bool>>, // 控制是否继续缓存 Hello 消息（HelloEnd 后停止）
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
            audio_callback: None,
            asr_callback: None,
            raw_message_callback: None,
            cached_hello_messages: Arc::new(RwLock::new(Vec::new())),
            pending_hello_sessions: Arc::new(RwLock::new(Vec::new())),
            hello_caching_enabled: Arc::new(RwLock::new(true)), // 初始启用缓存
        }
    }

    /// Create a new EchoKitClient with audio callback support
    pub fn new_with_audio_callback(
        websocket_url: String,
        audio_callback: mpsc::UnboundedSender<(String, Vec<u8>)>,
    ) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();

        Self {
            websocket_url,
            ws_stream: Arc::new(RwLock::new(None)),
            is_connected: Arc::new(RwLock::new(false)),
            service_status: Arc::new(RwLock::new(None)),
            message_sender: tx,
            message_receiver: Arc::new(RwLock::new(Some(rx))),
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            audio_callback: Some(audio_callback),
            asr_callback: None,
            raw_message_callback: None,
            cached_hello_messages: Arc::new(RwLock::new(Vec::new())),
            pending_hello_sessions: Arc::new(RwLock::new(Vec::new())),
            hello_caching_enabled: Arc::new(RwLock::new(true)), // 初始启用缓存
        }
    }

    /// Create a new EchoKitClient with both audio and ASR callback support
    pub fn new_with_callbacks(
        websocket_url: String,
        audio_callback: mpsc::UnboundedSender<(String, Vec<u8>)>,
        asr_callback: mpsc::UnboundedSender<(String, String)>,
    ) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();

        Self {
            websocket_url,
            ws_stream: Arc::new(RwLock::new(None)),
            is_connected: Arc::new(RwLock::new(false)),
            service_status: Arc::new(RwLock::new(None)),
            message_sender: tx,
            message_receiver: Arc::new(RwLock::new(Some(rx))),
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            audio_callback: Some(audio_callback),
            asr_callback: Some(asr_callback),
            raw_message_callback: None,
            cached_hello_messages: Arc::new(RwLock::new(Vec::new())),
            pending_hello_sessions: Arc::new(RwLock::new(Vec::new())),
            hello_caching_enabled: Arc::new(RwLock::new(true)), // 初始启用缓存
        }
    }

    /// Create a new EchoKitClient with audio, ASR, and raw message callback support
    pub fn new_with_all_callbacks(
        websocket_url: String,
        audio_callback: mpsc::UnboundedSender<(String, Vec<u8>)>,
        asr_callback: mpsc::UnboundedSender<(String, String)>,
        raw_message_callback: mpsc::UnboundedSender<(String, Vec<u8>)>,
    ) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();

        Self {
            websocket_url,
            ws_stream: Arc::new(RwLock::new(None)),
            is_connected: Arc::new(RwLock::new(false)),
            service_status: Arc::new(RwLock::new(None)),
            message_sender: tx,
            message_receiver: Arc::new(RwLock::new(Some(rx))),
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            audio_callback: Some(audio_callback),
            asr_callback: Some(asr_callback),
            raw_message_callback: Some(raw_message_callback),
            cached_hello_messages: Arc::new(RwLock::new(Vec::new())),
            pending_hello_sessions: Arc::new(RwLock::new(Vec::new())),
            hello_caching_enabled: Arc::new(RwLock::new(true)), // 初始启用缓存
        }
    }

    // 连接到 EchoKit Server
    pub async fn connect(&self) -> Result<()> {
        self.connect_with_device_id(None).await
    }

    /// 连接到 EchoKit Server，支持动态 device_id 替换
    pub async fn connect_with_device_id(&self, device_id: Option<&str>) -> Result<()> {
        // 如果提供了 device_id，则替换 URL 中的 {device_id} 占位符
        let url_string = if let Some(id) = device_id {
            self.websocket_url.replace("{device_id}", id)
        } else {
            // 如果没有提供 device_id，使用默认值 "ci-test-visitor"
            self.websocket_url.replace("{device_id}", "ci-test-visitor")
        };

        let url = Url::parse(&url_string)
            .with_context(|| format!("Invalid WebSocket URL: {}", url_string))?;

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

        // 记录会话信息（仅当尚未注册时才插入，避免覆盖 pre_register_session 的注册）
        if let EchoKitClientMessage::StartSession { session_id, device_id, .. } = &message {
            let mut sessions = self.active_sessions.write().await;
            if !sessions.contains_key(session_id) {
                info!("🔑 Registering session {} in active_sessions (from send_message)", session_id);
                sessions.insert(session_id.clone(), device_id.clone());
                let count = sessions.len();
                info!("📊 Active sessions count after insert: {}", count);
            } else {
                info!("✅ Session {} already registered (pre-registered)", session_id);
            }
        }

        // 实现WebSocket消息发送
        let json_message = serde_json::to_string(&message)
            .with_context(|| "Failed to serialize message")?;

        info!("📤 Sending message to EchoKit Server: {}", json_message);

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

    // 🔑 预注册会话（在 start_session 之前调用）
    // 这样可以确保当 HelloChunk 到达时，active_sessions 已经有该会话
    pub async fn pre_register_session(&self, session_id: String, device_id: String) {
        info!(
            "🔑 Pre-registering session {} for device {} in active_sessions",
            session_id, device_id
        );
        self.active_sessions.write().await.insert(session_id.clone(), device_id);
        let count = self.active_sessions.read().await.len();
        info!("📊 Active sessions count after pre-register: {}", count);

        // 🎁 将会话加入待发送缓存 Hello 的列表
        // 实际发送会在首次接收到该会话的消息处理请求时进行
        self.pending_hello_sessions.write().await.push(session_id.clone());
        info!("📝 Session {} added to pending hello list", session_id);
    }

    // 🎁 检查并发送缓存的 Hello 消息给指定会话（如果是首次）
    pub async fn check_and_send_cached_hello(&self, session_id: &str) {
        // 检查是否在待发送列表中
        let mut pending = self.pending_hello_sessions.write().await;
        if let Some(pos) = pending.iter().position(|s| s == session_id) {
            // 从待发送列表中移除
            pending.remove(pos);
            drop(pending); // 释放锁

            info!("🎁 Session {} ready for cached Hello messages", session_id);

            let cached_messages = self.cached_hello_messages.read().await;
            if cached_messages.is_empty() {
                info!("⚠️ No cached Hello messages to send to session {}", session_id);
                return;
            }

            info!("🎁 Sending {} cached Hello messages to session {}", cached_messages.len(), session_id);

            if let Some(callback) = &self.raw_message_callback {
                for (i, data) in cached_messages.iter().enumerate() {
                    info!("📤 Forwarding cached Hello message {} ({} bytes) to session {}", i + 1, data.len(), session_id);
                    if let Err(e) = callback.send((session_id.to_string(), data.clone())) {
                        error!("❌ Failed to send cached Hello message to session {}: {}", session_id, e);
                    } else {
                        info!("✅ Cached Hello message {} forwarded successfully", i + 1);
                    }

                    // 添加小延迟，确保每条消息作为独立的 WebSocket 帧发送
                    // 避免多条消息在网络层被合并
                    // 优化：从 10ms 减少到 3ms，减少总延迟
                    tokio::time::sleep(tokio::time::Duration::from_millis(3)).await;
                }
            } else {
                warn!("⚠️ No raw message callback available for sending cached Hello messages");
            }
        }
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

    // 发送音频数据（直接发送二进制，不使用JSON）
    pub async fn send_audio_data(
        &self,
        session_id: String,
        device_id: String,
        audio_data: Vec<u8>,
        format: AudioFormat,
        is_final: bool,
    ) -> Result<()> {
        if !self.is_connected().await {
            return Err(anyhow::anyhow!("Not connected to EchoKit Server"));
        }

        info!(
            "📤 Sending audio data: {} bytes (format: {:?}, final: {}) for session {}",
            audio_data.len(),
            format,
            is_final,
            session_id
        );

        // 直接发送二进制音频数据（不使用JSON）
        // EchoKit Server期望16-bit PCM音频作为Binary WebSocket消息
        let mut ws_stream_guard = self.ws_stream.write().await;
        if let Some(ws_stream) = ws_stream_guard.as_mut() {
            if let Err(e) = ws_stream.send(Message::Binary(audio_data.clone())).await {
                error!("Failed to send audio data to EchoKit Server: {}", e);
                *self.is_connected.write().await = false;
                return Err(anyhow::anyhow!("WebSocket send error: {}", e));
            }
            info!("✅ Audio data sent successfully to EchoKit Server");
        } else {
            return Err(anyhow::anyhow!("WebSocket stream not available"));
        }

        Ok(())
    }

    // 发送StartChat命令（通知EchoKit开始对话）
    pub async fn send_start_chat_command(&self) -> Result<()> {
        if !self.is_connected().await {
            return Err(anyhow::anyhow!("Not connected to EchoKit Server"));
        }

        info!("📤 Sending StartChat command to EchoKit Server");

        // 发送StartChat JSON消息
        let start_chat_message = serde_json::json!({"event": "StartChat"});
        let json_message = serde_json::to_string(&start_chat_message)
            .with_context(|| "Failed to serialize StartChat message")?;

        let mut ws_stream_guard = self.ws_stream.write().await;
        if let Some(ws_stream) = ws_stream_guard.as_mut() {
            if let Err(e) = ws_stream.send(Message::Text(json_message)).await {
                error!("Failed to send StartChat command to EchoKit Server: {}", e);
                *self.is_connected.write().await = false;
                return Err(anyhow::anyhow!("WebSocket send error: {}", e));
            }
            info!("✅ StartChat command sent successfully to EchoKit Server");
        } else {
            return Err(anyhow::anyhow!("WebSocket stream not available"));
        }

        Ok(())
    }

    // 发送Submit命令（通知EchoKit处理音频）
    pub async fn send_submit_command(&self) -> Result<()> {
        if !self.is_connected().await {
            return Err(anyhow::anyhow!("Not connected to EchoKit Server"));
        }

        info!("📤 Sending Submit command to EchoKit Server");

        // 发送Submit JSON消息
        let submit_message = serde_json::json!({"event": "Submit"});
        let json_message = serde_json::to_string(&submit_message)
            .with_context(|| "Failed to serialize Submit message")?;

        let mut ws_stream_guard = self.ws_stream.write().await;
        if let Some(ws_stream) = ws_stream_guard.as_mut() {
            if let Err(e) = ws_stream.send(Message::Text(json_message)).await {
                error!("Failed to send Submit command to EchoKit Server: {}", e);
                *self.is_connected.write().await = false;
                return Err(anyhow::anyhow!("WebSocket send error: {}", e));
            }
            info!("✅ Submit command sent successfully to EchoKit Server");
        } else {
            return Err(anyhow::anyhow!("WebSocket stream not available"));
        }

        Ok(())
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
        let audio_callback = self.audio_callback.clone();
        let asr_callback = self.asr_callback.clone();
        let raw_message_callback = self.raw_message_callback.clone();
        let cached_hello_messages = self.cached_hello_messages.clone();
        let pending_hello_sessions = self.pending_hello_sessions.clone();
        let hello_caching_enabled = self.hello_caching_enabled.clone();

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
                                info!("📩 Received text message from EchoKit Server: {}", text);
                                if let Err(e) = Self::handle_server_message(
                                    text,
                                    &service_status,
                                    &active_sessions,
                                    &asr_callback,
                                    &hello_caching_enabled,
                                ).await {
                                    error!("Error handling server message: {}", e);
                                }
                            }
                            Some(Ok(Message::Binary(data))) => {
                                info!("📦 Received binary data from EchoKit Server: {} bytes", data.len());

                                // 首先尝试作为MessagePack解析
                                match rmpv::decode::read_value(&mut &data[..]) {
                                    Ok(msgpack_value) => {
                                        info!("📦 Parsed as MessagePack: {:?}", msgpack_value);

                                        // 🎁 检查是否是 Hello 相关消息，如果是则缓存
                                        let should_cache = Self::should_cache_hello_message(&msgpack_value);
                                        if should_cache && *hello_caching_enabled.read().await {
                                            info!("🎁 Caching Hello-related message ({} bytes)", data.len());
                                            cached_hello_messages.write().await.push(data.clone());
                                            let cache_size = cached_hello_messages.read().await.len();
                                            info!("📦 Cached messages count: {}", cache_size);
                                        } else if should_cache {
                                            info!("⏹️ Skipping Hello message caching (disabled after HelloEnd)");
                                        }

                                        // 对于所有MessagePack消息，直接转发原始数据给所有活跃会话
                                        // 客户端会自己解析MessagePack
                                        let sessions = active_sessions.read().await;
                                        info!("📊 Active sessions count: {}", sessions.len());
                                        for (session_id, _) in sessions.iter() {
                                            // 直接发送当前消息（Hello 消息已在 register_bridge_session 时发送）
                                            if let Some(callback) = &audio_callback {
                                                info!("📤 Forwarding MessagePack data to session: {}", session_id);
                                                if let Err(e) = callback.send((session_id.clone(), data.clone())) {
                                                    error!("❌ Failed to forward MessagePack to session {}: {}", session_id, e);
                                                } else {
                                                    info!("✅ MessagePack forwarded successfully to session {}", session_id);
                                                }
                                            } else {
                                                warn!("⚠️ No audio callback available for forwarding");
                                            }
                                        }

                                        // 额外处理ASR事件，用于日志记录和其他内部逻辑
                                        if let Err(e) = Self::handle_messagepack_data(
                                            msgpack_value,
                                            &active_sessions,
                                            &audio_callback,
                                            &asr_callback,
                                            &cached_hello_messages,
                                            &hello_caching_enabled,
                                        ).await {
                                            warn!("Error handling MessagePack data: {}", e);
                                        }
                                    }
                                    Err(_) => {
                                        // 不是MessagePack，当作原始音频数据处理
                                        if let Err(e) = Self::handle_binary_audio_data(
                                            data,
                                            &service_status,
                                            &active_sessions,
                                            &audio_callback,
                                        ).await {
                                            error!("Error handling binary audio data: {}", e);
                                        }
                                    }
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
        asr_callback: &Option<mpsc::UnboundedSender<(String, String)>>,
        hello_caching_enabled: &Arc<RwLock<bool>>,
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
                device_id: _,
                text,
                confidence,
                is_final,
                timestamp: _
            } => {
                info!("📝 Received Transcription for session {}: {} (confidence: {:.2}, final: {})",
                      session_id, text, confidence, is_final);

                // Forward ASR results via callback if available
                if let Some(callback) = asr_callback {
                    info!("Attempting to forward ASR via callback...");
                    if let Err(e) = callback.send((session_id.clone(), text.clone())) {
                        error!("❌ Failed to send ASR result via callback: {}", e);
                    } else {
                        info!("✅ Successfully forwarded ASR result for session {} to callback", session_id);
                    }
                } else {
                    warn!("⚠️ No ASR callback available to forward transcription");
                }
            }
            EchoKitServerMessage::Response {
                session_id,
                device_id: _,
                text,
                audio_data,
                is_complete,
                timestamp: _
            } => {
                info!("Response for session {}: {} (complete: {})", session_id, text, is_complete);
                if let Some(audio) = audio_data {
                    debug!("Received audio data: {} bytes", audio.len());
                }
                // 这里可以转发响应到设备
            }
            EchoKitServerMessage::Error { session_id, device_id: _, error } => {
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

    /// Create a new connection manager with audio callback support
    pub fn new_with_audio_callback(
        websocket_url: String,
        audio_callback: mpsc::UnboundedSender<(String, Vec<u8>)>,
    ) -> Self {
        Self {
            client: Arc::new(EchoKitClient::new_with_audio_callback(websocket_url, audio_callback)),
            reconnect_interval: tokio::time::Duration::from_secs(5),
            max_reconnect_attempts: 10,
        }
    }

    /// Create a new connection manager with both audio and ASR callback support
    pub fn new_with_callbacks(
        websocket_url: String,
        audio_callback: mpsc::UnboundedSender<(String, Vec<u8>)>,
        asr_callback: mpsc::UnboundedSender<(String, String)>,
    ) -> Self {
        Self {
            client: Arc::new(EchoKitClient::new_with_callbacks(websocket_url, audio_callback, asr_callback)),
            reconnect_interval: tokio::time::Duration::from_secs(5),
            max_reconnect_attempts: 10,
        }
    }

    /// Create a new connection manager with audio, ASR, and raw message callback support
    pub fn new_with_all_callbacks(
        websocket_url: String,
        audio_callback: mpsc::UnboundedSender<(String, Vec<u8>)>,
        asr_callback: mpsc::UnboundedSender<(String, String)>,
        raw_message_callback: mpsc::UnboundedSender<(String, Vec<u8>)>,
    ) -> Self {
        Self {
            client: Arc::new(EchoKitClient::new_with_all_callbacks(
                websocket_url,
                audio_callback,
                asr_callback,
                raw_message_callback
            )),
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
    // 判断是否应该缓存 Hello 相关消息
    fn should_cache_hello_message(value: &rmpv::Value) -> bool {
        use rmpv::Value;

        match value {
            Value::String(s) => {
                let event_str = s.as_str().unwrap_or("");
                matches!(event_str, "HelloStart" | "HelloEnd")
            }
            Value::Map(entries) => {
                for (key, _) in entries {
                    if let Value::String(key_str) = key {
                        let event_type = key_str.as_str().unwrap_or("");
                        if event_type == "HelloChunk" {
                            return true;
                        }
                    }
                }
                false
            }
            _ => false,
        }
    }

    // 处理MessagePack格式的数据（可能包含ASR等事件）
    async fn handle_messagepack_data(
        value: rmpv::Value,
        active_sessions: &Arc<RwLock<HashMap<String, String>>>,
        audio_callback: &Option<mpsc::UnboundedSender<(String, Vec<u8>)>>,
        asr_callback: &Option<mpsc::UnboundedSender<(String, String)>>,
        cached_hello_messages: &Arc<RwLock<Vec<Vec<u8>>>>,
        hello_caching_enabled: &Arc<RwLock<bool>>,
    ) -> Result<()> {
        use rmpv::Value;

        // MessagePack可能是字符串事件或对象事件
        match value {
            Value::String(s) => {
                let event_str = s.into_str().unwrap_or_default();
                info!("📦 MessagePack string event: {}", event_str);

                // 处理字符串事件如 "HelloStart", "HelloEnd", "EndAudio" 等
                // 这些事件需要通过特定的格式发送给客户端
                match event_str.as_str() {
                    "HelloStart" => {
                        info!("🎯 Received HelloStart - clearing cached Hello messages");
                        // 清空之前的缓存，准备缓存新的 Hello 序列
                        cached_hello_messages.write().await.clear();

                        // 🔓 启用缓存（新的问候序列开始）
                        *hello_caching_enabled.write().await = true;

                        info!("🎯 Forwarding event to clients: {}", event_str);
                        // ✅ 使用 MessagePack 编码（保持与 EchoKit 原始格式一致）
                        // 直接编码字符串 "HelloStart"，与 EchoKit Server 发送的格式相同
                        let event_bytes = rmp_serde::to_vec(&event_str)
                            .expect("Failed to serialize HelloStart to MessagePack");

                        // 缓存 HelloStart
                        cached_hello_messages.write().await.push(event_bytes.clone());

                        // 转发到所有活跃会话
                        let sessions = active_sessions.read().await;
                        for (session_id, _) in sessions.iter() {
                            if let Some(callback) = audio_callback {
                                info!("📤 Forwarding {} event to session: {}", event_str, session_id);
                                if let Err(e) = callback.send((session_id.clone(), event_bytes.clone())) {
                                    error!("❌ Failed to send {} event to session {}: {}", event_str, session_id, e);
                                } else {
                                    info!("✅ Successfully forwarded {} event to session {}", event_str, session_id);
                                }
                            }
                        }
                    }
                    "HelloEnd" => {
                        info!("🎯 Received HelloEnd - finalizing cached Hello messages");

                        // ✅ HelloEnd 已经在前面的通用缓存逻辑中被缓存了（line 507），这里不需要重复缓存
                        // 只需要记录日志和转发给活跃会话即可

                        // ✅ 使用 MessagePack 编码（保持与 EchoKit 原始格式一致）
                        let event_bytes = rmp_serde::to_vec(&event_str)
                            .expect("Failed to serialize HelloEnd to MessagePack");

                        let cached_messages = cached_hello_messages.read().await;
                        let cache_size = cached_messages.len();
                        let total_bytes: usize = cached_messages.iter().map(|msg| msg.len()).sum();
                        let estimated_seconds = total_bytes as f64 / (16000.0 * 2.0); // 16kHz, 16-bit
                        info!("🎁 Greeting cached: {} chunks (including HelloEnd), ~{:.1} seconds audio, {} bytes total, ready for instant delivery",
                            cache_size, estimated_seconds, total_bytes);

                        // 🔒 禁用缓存（问候序列已结束，不再缓存后续的 Hello 消息）
                        *hello_caching_enabled.write().await = false;
                        info!("⏹️ Hello message caching disabled after HelloEnd");

                        info!("🎯 Forwarding event to clients: {}", event_str);

                        // 转发到所有活跃会话
                        let sessions = active_sessions.read().await;
                        for (session_id, _) in sessions.iter() {
                            if let Some(callback) = audio_callback {
                                info!("📤 Forwarding {} event to session: {}", event_str, session_id);
                                if let Err(e) = callback.send((session_id.clone(), event_bytes.clone())) {
                                    error!("❌ Failed to send {} event to session {}: {}", event_str, session_id, e);
                                } else {
                                    info!("✅ Successfully forwarded {} event to session {}", event_str, session_id);
                                }
                            }
                        }
                    }
                    "EndAudio" | "EndResponse" => {
                        info!("🎯 Forwarding event to clients: {}", event_str);

                        // ✅ 使用 MessagePack 编码（保持与 EchoKit 原始格式一致）
                        let event_bytes = rmp_serde::to_vec(&event_str)
                            .expect(&format!("Failed to serialize {} to MessagePack", event_str));

                        // 转发到所有活跃会话
                        let sessions = active_sessions.read().await;
                        for (session_id, _) in sessions.iter() {
                            if let Some(callback) = audio_callback {
                                info!("📤 Forwarding {} event to session: {}", event_str, session_id);
                                if let Err(e) = callback.send((session_id.clone(), event_bytes.clone())) {
                                    error!("❌ Failed to send {} event to session {}: {}", event_str, session_id, e);
                                } else {
                                    info!("✅ Successfully forwarded {} event to session {}", event_str, session_id);
                                }
                            }
                        }
                    }
                    _ => {
                        debug!("📦 Unhandled string event: {}", event_str);
                    }
                }
            }
            Value::Map(entries) => {
                // 对象事件，如 {ASR: ["转录文本"]}, {HelloChunk: [音频数据]}
                for (key, val) in entries {
                    if let Value::String(key_str) = key {
                        let event_type = key_str.into_str().unwrap_or_default();
                        info!("📦 MessagePack object event: {}", event_type);

                        match event_type.as_str() {
                            "ASR" => {
                                // ASR事件：仅用于服务器端日志记录
                                // 注意：ASR 数据已经通过 audio_callback 作为原始 MessagePack 转发给客户端
                                // 这里不再重复转发，只记录日志用于服务器监控
                                if let Value::Array(arr) = val {
                                    if let Some(Value::String(text_val)) = arr.first() {
                                        let asr_text = text_val.as_str().unwrap_or("");
                                        info!("📝 Received ASR from EchoKit: {}", asr_text);

                                        // 仅用于内部监控和调试，不再转发
                                        debug!("� ASR text for monitoring: {}", asr_text);
                                    }
                                }
                            }
                            "HelloChunk" | "AudioChunk" => {
                                // 音频块事件：提取音频数据
                                if let Value::Array(arr) = val {
                                    if let Some(Value::Binary(audio_data)) = arr.first() {
                                                                                info!("👋 Received {} from EchoKit: {} bytes", event_type, audio_data.len());

                                        // 注意：音频数据已经通过 audio_callback 作为原始 MessagePack 转发
                                        // 这里不再重复转发，仅保留日志记录

                                        // 转发音频数据到所有活跃会话
                                        let sessions = active_sessions.read().await;
                                        for (session_id, _) in sessions.iter() {
                                            if let Some(callback) = audio_callback {
                                                info!("� Forwarding {} to session: {}", event_type, session_id);
                                                if let Err(e) = callback.send((session_id.clone(), audio_data.clone())) {
                                                    error!("❌ Failed to send {} to session {}: {}", event_type, session_id, e);
                                                } else {
                                                    debug!("✅ Successfully forwarded {} to session {}", event_type, session_id);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            "StartAudio" => {
                                info!("🔊 Start audio event");

                                // 转发 StartAudio 事件
                                let event_json = serde_json::json!({
                                    "event": "StartAudio"
                                }).to_string();
                                let event_bytes = event_json.as_bytes().to_vec();

                                let sessions = active_sessions.read().await;
                                for (session_id, _) in sessions.iter() {
                                    if let Some(callback) = audio_callback {
                                        let _ = callback.send((session_id.clone(), event_bytes.clone()));
                                    }
                                }
                            }
                            _ => {
                                debug!("📦 Unhandled MessagePack event: {}", event_type);
                            }
                        }
                    }
                }
            }
            _ => {
                debug!("📦 Unexpected MessagePack value type: {:?}", value);
            }
        }

        Ok(())
    }

    // 处理二进制音频数据
    async fn handle_binary_audio_data(
        data: Vec<u8>,
        _service_status: &Arc<RwLock<Option<EchoKitServiceStatus>>>,
        active_sessions: &Arc<RwLock<HashMap<String, String>>>,
        audio_callback: &Option<mpsc::UnboundedSender<(String, Vec<u8>)>>,
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

        // 如果有音频回调，将音频数据路由到相应的会话
        if let Some(callback) = audio_callback {
            // 获取所有活跃会话（这里需要从数据中确定session_id）
            // 由于当前没有在二进制数据中包含session_id，我们需要从活跃会话中找到
            // 这是一个临时方案，实际应该在数据中包含session_id
            let sessions = active_sessions.read().await;

            // 暂时发送给所有活跃会话（需要优化）
            for (session_id, _device_id) in sessions.iter() {
                if let Err(e) = callback.send((session_id.clone(), data.clone())) {
                    error!("Failed to send audio to session {}: {}", session_id, e);
                }
            }
        }

        info!("Audio data processed successfully (format: {}, size: {} bytes)",
              audio_format, data.len());

        Ok(())
    }
}
