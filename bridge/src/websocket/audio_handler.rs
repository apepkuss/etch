use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State, Path, Query,
    },
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use std::collections::HashMap;
use tracing::{debug, error, info, warn};

use crate::echokit::EchoKitSessionAdapter;
use super::connection_manager::DeviceConnectionManager;
use super::session_manager::SessionManager;

/// 应用状态
#[derive(Clone)]
pub struct AppState {
    pub connection_manager: Arc<DeviceConnectionManager>,
    pub session_manager: Arc<SessionManager>,
    pub echokit_adapter: Arc<EchoKitSessionAdapter>,
}

/// WebSocket 升级处理器
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> Response {
    // TODO: 验证设备 Token
    // 临时：生成随机 device_id
    let device_id = format!("device_{}", uuid::Uuid::new_v4());

    info!("Device {} initiating WebSocket connection", device_id);

    ws.on_upgrade(move |socket| handle_device_websocket(socket, device_id, false, state))
}

/// WebSocket 升级处理器（带 visitor_id 和 record 参数）
pub async fn websocket_handler_with_id(
    ws: WebSocketUpgrade,
    Path(visitor_id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    State(state): State<AppState>,
) -> Response {
    // 从查询参数中提取 record 模式
    let record_mode = params
        .get("record")
        .map(|v| v == "true")
        .unwrap_or(false);

    info!(
        "Client {} connecting (record_mode: {})",
        visitor_id, record_mode
    );

    ws.on_upgrade(move |socket| {
        handle_device_websocket(socket, visitor_id, record_mode, state)
    })
}

/// 处理设备 WebSocket 连接
async fn handle_device_websocket(
    socket: WebSocket,
    device_id: String,
    record_mode: bool,
    state: AppState,
) {
    let (sender, mut receiver) = socket.split();

    // 1. 注册设备连接
    if let Err(e) = state.connection_manager
        .register_device(device_id.clone(), sender)
        .await
    {
        error!("Failed to register device {}: {}", device_id, e);
        return;
    }

    info!("Device {} WebSocket connected (record_mode: {})", device_id, record_mode);

    // 2. 当前活跃会话 ID
    let mut active_session: Option<String> = None;

    // 🔧 用于跟踪设备级别的 EchoKit 会话（避免重复创建）
    let mut device_echokit_session: Option<String> = None;

    // 3. 处理设备消息
    while let Some(msg_result) = receiver.next().await {
        match msg_result {
            Ok(Message::Text(text)) => {
                // 更新心跳（任何客户端消息都表示连接活跃）
                state.connection_manager.update_heartbeat(&device_id).await;

                // 处理控制消息
                if let Err(e) = handle_control_message(
                    &text,
                    &device_id,
                    record_mode,
                    &mut active_session,
                    &mut device_echokit_session,
                    &state,
                ).await {
                    error!("Failed to handle control message: {}", e);
                }
            }

            Ok(Message::Binary(audio_data)) => {
                // 更新心跳（音频数据也表示连接活跃）
                state.connection_manager.update_heartbeat(&device_id).await;

                // 处理音频数据
                if let Some(session_id) = &active_session {
                    // ✅ 检查设备是否仍然连接
                    if !state.connection_manager.is_device_online(&device_id).await {
                        warn!(
                            "⚠️ Ignoring audio from disconnected device {} (session: {})",
                            device_id, session_id
                        );
                        break;
                    }

                    info!(
                        "📊 Received audio data: {} bytes for session {}",
                        audio_data.len(),
                        session_id
                    );

                    // 验证音频格式（16-bit PCM, 应该是偶数字节）
                    if audio_data.len() % 2 != 0 {
                        warn!("⚠️ Audio data length is odd: {} bytes (expecting 16-bit PCM)", audio_data.len());
                    }

                    // 采样率验证（假设1秒音频应该是32000字节 = 16000样本 * 2字节）
                    let estimated_samples = audio_data.len() / 2;
                    let estimated_duration_ms = (estimated_samples as f32 / 16.0) as u32; // 16样本/ms @ 16kHz
                    info!(
                        "📊 Audio stats: ~{} samples, ~{}ms @ 16kHz",
                        estimated_samples,
                        estimated_duration_ms
                    );

                    if let Err(e) = forward_audio_to_echokit(
                        session_id,
                        audio_data.to_vec(), // Convert Bytes to Vec<u8>
                        &state,
                    ).await {
                        error!("Failed to forward audio: {}", e);
                    }
                } else {
                    warn!("Received audio data without active session from device {}", device_id);
                }
            }

            Ok(Message::Ping(data)) => {
                // 响应 Ping 并更新心跳
                state.connection_manager.update_heartbeat(&device_id).await;
                if let Err(e) = state.connection_manager
                    .send_pong(&device_id, data.to_vec()) // Convert Bytes to Vec<u8>
                    .await
                {
                    error!("Failed to send pong: {}", e);
                }
            }

            Ok(Message::Close(_)) => {
                info!("Device {} closed WebSocket connection", device_id);
                break;
            }

            Err(e) => {
                error!("WebSocket error for device {}: {}", device_id, e);
                break;
            }

            _ => {}
        }
    }

    // 4. 清理连接
    if let Some(session_id) = active_session {
        let _ = state.session_manager.end_session(&session_id).await;
    }

    let _ = state.connection_manager.remove_device(&device_id).await;
    info!("Device {} disconnected", device_id);
}

/// 处理控制消息（JSON格式）
async fn handle_control_message(
    text: &str,
    device_id: &str,
    record_mode: bool,
    active_session: &mut Option<String>,
    device_echokit_session: &mut Option<String>,
    state: &AppState,
) -> anyhow::Result<()> {
    // 优先尝试解析为 ClientCommand（Web 客户端协议）
    if let Ok(cmd) = super::protocol::ClientCommand::from_json(text) {
        return handle_client_command(cmd, device_id, record_mode, active_session, device_echokit_session, state).await;
    }

    // 回退到旧的 DeviceEvent 格式（保持向后兼容）
    let event: DeviceEvent = serde_json::from_str(text)?;

    match event.event_type.as_str() {
        "start_session" => {
            // 创建新会话
            let session_id = generate_session_id();
            info!("Device {} starting session {}", device_id, session_id);

            // 绑定会话到设备
            state.session_manager
                .create_session(session_id.clone(), device_id.to_string())
                .await?;

            state.connection_manager
                .bind_session(session_id.clone(), device_id.to_string())
                .await?;

            // 创建 EchoKit 会话
            let echokit_config = echo_shared::EchoKitConfig::default();
            if let Err(e) = state.echokit_adapter
                .create_echokit_session(
                    session_id.clone(),
                    device_id.to_string(),
                    echokit_config,
                )
                .await
            {
                error!("Failed to create EchoKit session: {}", e);
                // 继续处理，但记录错误
            }

            // 更新活跃会话
            *active_session = Some(session_id.clone());

            // 响应设备
            let response = serde_json::json!({
                "event": "session_started",
                "session_id": session_id,
                "timestamp": chrono::Utc::now().timestamp()
            });

            state.connection_manager
                .send_text(device_id, &response.to_string())
                .await?;
        }

        "end_session" => {
            if let Some(session_id) = event.session_id {
                info!("Device {} ending session {}", device_id, session_id);

                // 关闭 EchoKit 会话
                if let Err(e) = state.echokit_adapter
                    .close_echokit_session(&session_id)
                    .await
                {
                    error!("Failed to close EchoKit session: {}", e);
                }

                state.session_manager.end_session(&session_id).await?;
                state.connection_manager.unbind_session(&session_id).await?;
                *active_session = None;

                // 响应设备
                let response = serde_json::json!({
                    "event": "session_ended",
                    "session_id": session_id
                });

                state.connection_manager
                    .send_text(device_id, &response.to_string())
                    .await?;
            }
        }

        "heartbeat" => {
            // 心跳响应
            state.connection_manager.update_heartbeat(device_id).await;

            let response = serde_json::json!({
                "event": "heartbeat_ack",
                "timestamp": chrono::Utc::now().timestamp()
            });

            state.connection_manager
                .send_text(device_id, &response.to_string())
                .await?;
        }

        _ => {
            warn!("Unknown event type: {}", event.event_type);
        }
    }

    Ok(())
}

/// 转发音频到 EchoKit
async fn forward_audio_to_echokit(
    session_id: &str,
    audio_data: Vec<u8>,
    state: &AppState,
) -> anyhow::Result<()> {
    let data_len = audio_data.len();

    // 🔑 关键修复：在转发音频前，确保本轮对话已发送 StartChat
    // 检查当前session是否需要发送StartChat（每轮对话的第一个音频包）
    let needs_start_chat = state.session_manager.needs_start_chat_for_round(session_id).await;

    if needs_start_chat {
        info!("🎬 Detected new conversation round for session {}, sending StartChat", session_id);

        // 发送 StartChat 命令到 EchoKit Server
        if let Err(e) = state.echokit_adapter.send_start_chat_for_session(session_id).await {
            error!("Failed to send StartChat for session {}: {}", session_id, e);
            return Err(e.into());
        }

        // 标记本轮已发送 StartChat
        state.session_manager.mark_start_chat_sent(session_id).await;
        info!("✅ StartChat sent for new conversation round (session: {})", session_id);
    }

    // 使用 EchoKit 适配器转发音频
    state.echokit_adapter
        .forward_audio(session_id, audio_data)
        .await?;

    // 更新会话统计
    state.session_manager.increment_sent_frames(session_id).await;

    debug!("Forwarded {} bytes audio for session {}", data_len, session_id);
    Ok(())
}

/// 处理客户端命令（Web 客户端协议）
async fn handle_client_command(
    cmd: super::protocol::ClientCommand,
    device_id: &str,
    record_mode: bool,
    active_session: &mut Option<String>,
    device_echokit_session: &mut Option<String>,
    state: &AppState,
) -> anyhow::Result<()> {
    use super::protocol::ClientCommand;

    match cmd {
        ClientCommand::StartChat | ClientCommand::StartRecord => {
            // 使用传入的 record_mode 参数，或从命令判断（向后兼容）
            let is_record = record_mode || cmd.is_record_mode();

            // 如果已有活跃会话，先清理（支持多轮对话）
            if let Some(old_session_id) = active_session.take() {
                info!(
                    "🔄 Device {} starting new session, cleaning up old session {}",
                    device_id, old_session_id
                );

                // 关闭旧的 EchoKit 会话
                if let Err(e) = state.echokit_adapter
                    .close_echokit_session(&old_session_id)
                    .await
                {
                    error!("Failed to close old EchoKit session: {}", e);
                }

                // 清理旧会话
                if let Err(e) = state.session_manager.end_session(&old_session_id).await {
                    error!("Failed to end old session: {}", e);
                }
                if let Err(e) = state.connection_manager.unbind_session(&old_session_id).await {
                    error!("Failed to unbind old session: {}", e);
                }
            }

            // 创建新会话
            let session_id = generate_session_id();
            info!(
                "Device {} starting {} session {}",
                device_id,
                if is_record { "record" } else { "chat" },
                session_id
            );

            // 绑定会话到设备
            state.session_manager
                .create_session(session_id.clone(), device_id.to_string())
                .await?;

            state.connection_manager
                .bind_session(session_id.clone(), device_id.to_string())
                .await?;

            // 只有对话模式才创建 EchoKit 会话
            if !is_record {
                let echokit_config = echo_shared::EchoKitConfig::default();

                // 🔧 检查是否已有设备级别的 EchoKit 会话
                if let Some(existing_ek_session) = &device_echokit_session {
                    // 复用现有的 EchoKit 会话
                    info!(
                        "♻️ Reusing existing EchoKit session {} for bridge session {}",
                        existing_ek_session, session_id
                    );

                    // 将新的 bridge session 绑定到现有的 EchoKit 会话
                    state.echokit_adapter
                        .register_bridge_session(
                            session_id.clone(),
                            device_id.to_string(),
                            existing_ek_session.clone(),
                        )
                        .await?;

                    info!("✅ Bridge session {} bound to existing EchoKit session {}",
                          session_id, existing_ek_session);

                    // 🔑 关键修复：每轮对话都需要发送 StartChat 命令
                    // EchoKit Server 期望在每轮对话开始时收到 StartChat
                    if matches!(cmd, ClientCommand::StartChat) {
                        if let Err(e) = state.echokit_adapter.send_start_chat(&existing_ek_session).await {
                            error!("Failed to send StartChat command to EchoKit: {}", e);
                        } else {
                            info!("📤 StartChat command sent to EchoKit for session {}", existing_ek_session);
                        }
                    }
                } else {
                    // 首次创建 EchoKit 会话
                    match state.echokit_adapter
                        .create_echokit_session(
                            session_id.clone(),
                            device_id.to_string(),
                            echokit_config,
                        )
                        .await
                    {
                        Err(e) => {
                            error!("Failed to create EchoKit session: {}", e);
                        }
                        Ok(echokit_session_id) => {
                            // EchoKit 会话创建成功
                            info!("🆕 EchoKit session {} created for bridge session {}",
                                  echokit_session_id, session_id);

                            // 保存设备级别的 EchoKit 会话 ID
                            *device_echokit_session = Some(echokit_session_id.clone());

                            // 转发 StartChat 命令给 EchoKit
                            if matches!(cmd, ClientCommand::StartChat) {
                                if let Err(e) = state.echokit_adapter.send_start_chat(&echokit_session_id).await {
                                    error!("Failed to send StartChat command to EchoKit: {}", e);
                                } else {
                                    info!("📤 StartChat command forwarded to EchoKit for session {}", echokit_session_id);
                                }
                            }
                        }
                    }
                }
            } else {
                info!("Record mode: skipping EchoKit session creation");
            }

            // 更新活跃会话
            *active_session = Some(session_id.clone());

            // 响应客户端（兼容 Web 客户端，不发送响应）
            // Web 客户端不期望响应消息
            info!("Session {} created successfully", session_id);
        }

        ClientCommand::Submit => {
            if let Some(session_id) = active_session {
                info!("Device {} submitted audio for session {}", device_id, session_id);

                // 通知EchoKit Server处理音频
                // EchoKit期望收到Submit消息来触发ASR处理
                if let Err(e) = state.echokit_adapter.submit_audio_for_processing(session_id).await {
                    error!("Failed to submit audio to EchoKit for processing: {}", e);
                }

                debug!("Audio submission completed for session {}", session_id);

                // 🔄 重置本轮对话的 StartChat 标记
                // 下一轮对话需要重新发送 StartChat
                state.session_manager.reset_start_chat_flag(session_id).await;
                debug!("🔄 Reset StartChat flag for next conversation round");

                // 注意：不在这里清理会话
                // 会话会在收到 EchoKit 的 EndAudio 或 EndResponse 事件后自动清理
                // 或者在下一次 StartChat/StartRecord 时创建新会话时清理旧会话
                // 这样可以确保客户端接收到完整的响应（ASR + 音频）
                info!("💡 Session {} remains active to receive responses", session_id);
            } else {
                warn!("Received Submit without active session from device {}", device_id);
            }
        }

        ClientCommand::Text { input } => {
            if let Some(session_id) = active_session {
                info!(
                    "Device {} sent text input for session {}: {}",
                    device_id, session_id, input
                );

                // TODO: 处理文本输入，发送到 EchoKit
                // 当前 EchoKit 适配器可能需要扩展以支持文本输入
                warn!("Text input handling not yet implemented");
            } else {
                warn!("Received Text without active session from device {}", device_id);
            }
        }
    }

    Ok(())
}

/// 生成会话ID
fn generate_session_id() -> String {
    format!("session_{}", uuid::Uuid::new_v4())
}

/// 设备事件消息
#[derive(Debug, serde::Deserialize)]
struct DeviceEvent {
    event_type: String,
    session_id: Option<String>,
    timestamp: Option<i64>,
}
