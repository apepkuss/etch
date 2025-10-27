use echo_shared::AppConfig;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use futures_util::{SinkExt, StreamExt};
use serde_json;
use tracing::{info, warn, error};
use std::time::Duration;

pub async fn connect_to_gateway(config: AppConfig) -> anyhow::Result<()> {
    info!("Connecting to API Gateway WebSocket...");

    let gateway_url = "ws://localhost:8080/ws";
    let (ws_stream, response) = connect_async(gateway_url).await?;

    info!("WebSocket connected to API Gateway");
    info!("Response status: {}", response.status());

    let (mut write, mut read) = ws_stream.split();

    // 启动心跳任务 - 暂时注释掉因为SplitSink不能clone
    // let write_clone = write.clone();
    // tokio::spawn(async move {
    //     let mut interval = tokio::time::interval(Duration::from_secs(30));
    //     loop {
    //         interval.tick().await;

    //         if let Err(e) = write_clone.send(Message::Text(
    //             serde_json::json!({
    //                 "type": "ping",
    //                 "timestamp": chrono::Utc::now().timestamp()
    //             }).to_string()
    //         )).await {
    //             error!("Failed to send ping: {}", e);
    //             break;
    //         }
    //     }
    // });

    // 处理接收到的消息
    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                if let Err(e) = handle_message(&text).await {
                    error!("Error handling message: {}", e);
                }
            }
            Ok(Message::Binary(data)) => {
                info!("Received binary data: {} bytes", data.len());
            }
            Ok(Message::Close(_)) => {
                info!("WebSocket connection closed");
                break;
            }
            Err(e) => {
                error!("WebSocket error: {}", e);
                break;
            }
            _ => {}
        }
    }

    Ok(())
}

async fn handle_message(message: &str) -> anyhow::Result<()> {
    info!("Received WebSocket message: {}", message);

    // 尝试解析JSON消息
    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(message) {
        if let Some(msg_type) = parsed.get("type").and_then(|v| v.as_str()) {
            match msg_type {
                "SystemNotification" => {
                    info!("Received system notification");
                }
                "DeviceStatusUpdate" => {
                    info!("Received device status update");
                }
                "SessionProgress" => {
                    info!("Received session progress update");
                }
                _ => {
                    warn!("Unknown message type: {}", msg_type);
                }
            }
        }
    }

    Ok(())
}