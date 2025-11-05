use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
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

    ws.on_upgrade(move |socket| handle_device_websocket(socket, device_id, state))
}

/// 处理设备 WebSocket 连接
async fn handle_device_websocket(
    socket: WebSocket,
    device_id: String,
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

    info!("Device {} WebSocket connected", device_id);

    // 2. 当前活跃会话 ID
    let mut active_session: Option<String> = None;

    // 3. 处理设备消息
    while let Some(msg_result) = receiver.next().await {
        match msg_result {
            Ok(Message::Text(text)) => {
                // 处理控制消息
                if let Err(e) = handle_control_message(
                    &text,
                    &device_id,
                    &mut active_session,
                    &state,
                ).await {
                    error!("Failed to handle control message: {}", e);
                }
            }

            Ok(Message::Binary(audio_data)) => {
                // 处理音频数据
                if let Some(session_id) = &active_session {
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
                // 响应 Ping
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
    active_session: &mut Option<String>,
    state: &AppState,
) -> anyhow::Result<()> {
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

    // 使用 EchoKit 适配器转发音频
    state.echokit_adapter
        .forward_audio(session_id, audio_data)
        .await?;

    // 更新会话统计
    state.session_manager.increment_sent_frames(session_id).await;

    debug!("Forwarded {} bytes audio for session {}", data_len, session_id);
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
