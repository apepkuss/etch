use axum::{
    extract::{
        ws::{WebSocket, Message},
        WebSocketUpgrade,
    },
    response::Response,
};
use echo_shared::{WebSocketMessage, DeviceStatus, SessionStage, NotificationLevel};
use futures::{sink::SinkExt, stream::StreamExt};
use serde_json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tracing::{info, warn, error};

// 广播通道类型
type BroadcastReceiver = broadcast::Receiver<WebSocketMessage>;
type Broadcaster = broadcast::Sender<WebSocketMessage>;

// WebSocket 连接管理器
#[derive(Clone)]
struct ConnectionManager {
    connections: Arc<RwLock<HashMap<String, Broadcaster>>>,
}

impl ConnectionManager {
    fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn add_connection(&self, user_id: String) -> Broadcaster {
        let (tx, _rx) = broadcast::channel(1000);
        let mut connections = self.connections.write().await;
        connections.insert(user_id, tx.clone());
        tx
    }

    async fn remove_connection(&self, user_id: &str) {
        let mut connections = self.connections.write().await;
        connections.remove(user_id);
    }

    async fn broadcast(&self, message: WebSocketMessage) {
        let connections = self.connections.read().await;
        for (_, tx) in connections.iter() {
            if let Err(e) = tx.send(message.clone()) {
                warn!("Failed to send message to connection: {}", e);
            }
        }
    }
}

pub async fn websocket_handler(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(handle_websocket)
}

async fn handle_websocket(socket: WebSocket) {
    let connection_manager = ConnectionManager::new();

    // TODO: 从 JWT token 中解析用户ID
    let user_id = "user001".to_string();
    info!("WebSocket connection established for user: {}", user_id);

    let broadcaster = connection_manager.add_connection(user_id.clone()).await;
    let mut rx = broadcaster.subscribe();

    let (mut sender, mut receiver) = socket.split();

    // 发送欢迎消息
    let welcome_message = WebSocketMessage::SystemNotification {
        level: NotificationLevel::Info,
        title: "连接成功".to_string(),
        message: "WebSocket 连接已建立，开始接收实时更新".to_string(),
    };

    if let Ok(text) = serde_json::to_string(&welcome_message) {
        if let Err(e) = sender.send(Message::Text(text)).await {
            warn!("Failed to send welcome message: {}", e);
            return;
        }
    }

    // 启动消息发送任务
    let mut sender_task = tokio::spawn(async move {
        while let Ok(message) = rx.recv().await {
            if let Ok(text) = serde_json::to_string(&message) {
                if sender.send(Message::Text(text)).await.is_err() {
                    break;
                }
            }
        }
    });

    // 处理接收到的消息
    let connection_manager_clone = connection_manager.clone();
    let user_id_clone = user_id.clone();
    let mut receiver_task = tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if let Err(e) = handle_client_message(&text, &connection_manager_clone).await {
                        error!("Error handling client message: {}", e);
                    }
                }
                Ok(Message::Close(_)) => {
                    info!("WebSocket connection closed by client");
                    break;
                }
                Err(e) => {
                    warn!("WebSocket error: {}", e);
                    break;
                }
                _ => {}
            }
        }
    });

    // 等待任一任务完成
    tokio::select! {
        _ = (&mut sender_task) => {
            info!("Sender task completed");
        }
        _ = (&mut receiver_task) => {
            info!("Receiver task completed");
        }
    }

    connection_manager.remove_connection(&user_id_clone).await;

    // 模拟发送一些实时更新
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));

        loop {
            interval.tick().await;

            // 模拟设备状态更新
            let device_status_update = WebSocketMessage::DeviceStatusUpdate {
                device_id: format!("dev{:03}", rand::random::<u32>() % 3 + 1),
                status: if rand::random::<f32>() > 0.5 {
                    DeviceStatus::Online
                } else {
                    DeviceStatus::Offline
                },
                timestamp: echo_shared::now_utc(),
            };

            if let Err(e) = broadcaster.send(device_status_update) {
                warn!("Failed to broadcast device status update: {}", e);
            }

            // 模拟会话进度更新
            if rand::random::<f32>() > 0.7 {
                let session_progress = WebSocketMessage::SessionProgress {
                    session_id: format!("sess{:03}", rand::random::<u32>() % 100 + 1),
                    device_id: format!("dev{:03}", rand::random::<u32>() % 3 + 1),
                    stage: SessionStage::Processing,
                    progress: rand::random::<f32>() * 100.0,
                    message: "正在处理语音命令...".to_string(),
                };

                if let Err(e) = broadcaster.send(session_progress) {
                    warn!("Failed to broadcast session progress: {}", e);
                }
            }
        }
    });
}

async fn handle_client_message(
    message: &str,
    connection_manager: &ConnectionManager,
) -> Result<(), Box<dyn std::error::Error>> {
    let parsed: serde_json::Value = serde_json::from_str(message)?;

    if let Some(msg_type) = parsed.get("type").and_then(|v| v.as_str()) {
        match msg_type {
            "ping" => {
                // 响应客户端 ping
                let pong_message = WebSocketMessage::SystemNotification {
                    level: NotificationLevel::Info,
                    title: "Pong".to_string(),
                    message: "服务器响应".to_string(),
                };

                // 广播 pong 消息（实际生产环境中应该只发送给特定客户端）
                // 这里简化为广播给所有客户端
                connection_manager.broadcast(pong_message).await;
            }
            _ => {
                warn!("Unknown message type: {}", msg_type);
            }
        }
    }

    Ok(())
}