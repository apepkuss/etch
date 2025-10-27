use echo_shared::AppConfig;
use tokio::net::UdpSocket;
use tracing::{info, error, Level};
use tracing_subscriber;
use std::sync::Arc;

mod audio;
mod session;
mod websocket;
mod mqtt;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_target(false)
        .init();

    info!("Echo Bridge Service starting...");

    // 创建配置
    let config = AppConfig::default();

    // 创建UDP socket用于接收音频流
    let udp_socket = Arc::new(UdpSocket::bind("0.0.0.0:8081").await?);
    info!("Bridge UDP listening on 0.0.0.0:8081");

    // 启动音频流处理
    let audio_socket = udp_socket.clone();
    let audio_config = config.clone();
    tokio::spawn(async move {
        if let Err(e) = audio::handle_audio_stream(audio_socket, audio_config).await {
            error!("Audio stream error: {}", e);
        }
    });

    // 启动WebSocket客户端连接到API Gateway
    let ws_config = config.clone();
    tokio::spawn(async move {
        if let Err(e) = websocket::connect_to_gateway(ws_config).await {
            error!("WebSocket connection error: {}", e);
        }
    });

    // 启动MQTT客户端
    let mqtt_config = config.clone();
    tokio::spawn(async move {
        if let Err(e) = mqtt::start_mqtt_client(mqtt_config).await {
            error!("MQTT client error: {}", e);
        }
    });

    info!("Bridge Service started successfully");

    // 保持主线程运行
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
    }
}