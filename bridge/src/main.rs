mod echokit_client;
mod echokit;
mod audio_processor;
mod udp_server;
mod mqtt_client;
mod websocket;

use anyhow::{Context, Result};
use echo_shared::{
    EchoKitConfig, AudioFormat, WebSocketMessage,
    generate_session_id, DeviceStatus, TopicFilter, QoS, WakeReason
};
use echo_shared::mqtt::MqttConfig;
use echo_shared::utils::now_utc;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{info, warn, error, debug};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use axum::{extract::State, response::Json, routing::get, Router};

// Bridge 服务配置
#[derive(Debug, Clone)]
struct BridgeConfig {
    pub udp_bind_address: String,
    pub echokit_websocket_url: String,
    pub api_gateway_websocket_url: String,
    pub max_sessions: u32,
    pub session_timeout_seconds: i64,
    pub heartbeat_interval_seconds: u64,
    pub mqtt_broker_host: String,
    pub mqtt_broker_port: u16,
}

impl Default for BridgeConfig {
    fn default() -> Self {
        Self {
            udp_bind_address: "0.0.0.0:8083".to_string(),
            echokit_websocket_url: "ws://echokit-server:9988/v1/realtime".to_string(),
            api_gateway_websocket_url: "ws://api-gateway:8080/ws".to_string(),
            max_sessions: 100,
            session_timeout_seconds: 300, // 5分钟
            heartbeat_interval_seconds: 30,
            mqtt_broker_host: "mqtt".to_string(),
            mqtt_broker_port: 1883,
        }
    }
}

// Bridge 服务主结构
struct BridgeService {
    config: BridgeConfig,
    echokit_manager: Arc<echokit_client::EchoKitConnectionManager>,
    audio_processor: Arc<audio_processor::AudioProcessor>,
    udp_server: Arc<udp_server::UdpAudioServer>,
    mqtt_client: Arc<mqtt_client::BridgeMqttClient>,
    active_sessions: Arc<RwLock<std::collections::HashMap<String, SessionInfo>>>,
    device_audio_output: mpsc::UnboundedSender<(String, Vec<u8>)>,
    // WebSocket 组件
    connection_manager: Arc<websocket::connection_manager::DeviceConnectionManager>,
    session_manager: Arc<websocket::session_manager::SessionManager>,
    heartbeat_monitor: Arc<websocket::heartbeat::HeartbeatMonitor>,
    flow_controller: Arc<websocket::flow_control::FlowController>,
    echokit_adapter: Arc<echokit::EchoKitSessionAdapter>,
}

// 会话信息
#[derive(Debug, Clone)]
struct SessionInfo {
    session_id: String,
    device_id: String,
    user_id: String,
    config: EchoKitConfig,
    start_time: chrono::DateTime<chrono::Utc>,
    last_activity: chrono::DateTime<chrono::Utc>,
    is_active: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting Echo Bridge Service...");

    // 加载配置
    let config = load_config().await?;
    info!("Bridge configuration: {:?}", config);

    // 创建设备音频输出通道
    let (audio_output_tx, audio_output_rx) = mpsc::unbounded_channel();

    // 创建 MQTT 配置
    let mqtt_config = MqttConfig {
        broker_host: config.mqtt_broker_host.clone(),
        broker_port: config.mqtt_broker_port,
        client_id: format!("bridge-{}", uuid::Uuid::new_v4()),
        username: std::env::var("MQTT_USERNAME").ok(),
        password: std::env::var("MQTT_PASSWORD").ok(),
        keep_alive: 60,
        clean_session: true,
        max_reconnect_attempts: 10,
        reconnect_interval_ms: 5000,
    };

    // 创建音频回调通道（用于 EchoKit -> Adapter -> Device 的音频路由）
    let (audio_callback_tx, audio_callback_rx) = tokio::sync::mpsc::unbounded_channel();

    // 创建 EchoKit 连接管理器（带音频回调）
    let echokit_manager = Arc::new(echokit_client::EchoKitConnectionManager::new_with_audio_callback(
        config.echokit_websocket_url.clone(),
        audio_callback_tx,
    ));

    // 创建音频处理器
    let audio_processor = Arc::new(audio_processor::AudioProcessor::new(
        echokit_manager.get_client(),
        audio_output_tx.clone(),
    ));

    // 创建 UDP 服务器
    let udp_server = Arc::new(udp_server::UdpAudioServer::new(
        &config.udp_bind_address,
        audio_processor.clone(),
    ).await?);

    // 创建 MQTT 客户端
    let (mqtt_client, mqtt_event_loop) = mqtt_client::BridgeMqttClient::new(mqtt_config)?;
    let mqtt_client_arc = Arc::new(mqtt_client);

    // 创建 WebSocket 组件
    let connection_manager = Arc::new(websocket::connection_manager::DeviceConnectionManager::new());
    let session_manager = Arc::new(websocket::session_manager::SessionManager::new());

    // 创建 EchoKit 适配器（带音频接收器）
    let echokit_adapter = Arc::new(echokit::EchoKitSessionAdapter::new(
        echokit_manager.get_client(),
        connection_manager.clone(),
        audio_callback_rx,
    ));

    // 启动 EchoKit 音频接收器
    let echokit_adapter_clone = echokit_adapter.clone();
    tokio::spawn(async move {
        echokit_adapter_clone.start_audio_receiver().await;
    });

    // 创建心跳监控
    let heartbeat_config = websocket::heartbeat::HeartbeatConfig::default();
    let heartbeat_monitor = Arc::new(websocket::heartbeat::HeartbeatMonitor::new(
        connection_manager.clone(),
        session_manager.clone(),
        heartbeat_config,
    ));

    // 创建流控管理器
    let flow_config = websocket::flow_control::FlowControlConfig::default();
    let flow_controller = Arc::new(websocket::flow_control::FlowController::new(flow_config));

    // 创建 Bridge 服务
    let bridge_service = BridgeService {
        config: config.clone(),
        echokit_manager: echokit_manager.clone(),
        audio_processor: audio_processor.clone(),
        udp_server: udp_server.clone(),
        mqtt_client: mqtt_client_arc.clone(),
        active_sessions: Arc::new(RwLock::new(std::collections::HashMap::new())),
        device_audio_output: audio_output_tx,
        connection_manager: connection_manager.clone(),
        session_manager: session_manager.clone(),
        heartbeat_monitor: heartbeat_monitor.clone(),
        flow_controller: flow_controller.clone(),
        echokit_adapter: echokit_adapter.clone(),
    };

    // 启动 MQTT 事件循环
    // 由于 start() 方法需要消费 self，我们需要创建一个新的客户端实例来运行事件循环
    // 这个实例与第一个客户端共享同一个 broker 连接配置
    let mqtt_config_for_event_loop = echo_shared::mqtt::MqttConfig {
        client_id: format!("bridge-{}", uuid::Uuid::new_v4()),
        broker_host: config.mqtt_broker_host.clone(),
        broker_port: config.mqtt_broker_port,
        username: None,
        password: None,
        keep_alive: 60,
        clean_session: true,
        max_reconnect_attempts: 5,
        reconnect_interval_ms: 5000,
    };
    let (mqtt_client_for_event_loop, mqtt_event_loop_for_start) =
        mqtt_client::BridgeMqttClient::new(mqtt_config_for_event_loop)?;

    info!("Starting MQTT client event loop...");
    tokio::spawn(async move {
        if let Err(e) = mqtt_client_for_event_loop.start(mqtt_event_loop_for_start).await {
            error!("MQTT client event loop error: {}", e);
        }
    });

    // 启动各个组件
    bridge_service.start(audio_output_rx).await?;

    info!("Echo Bridge Service started successfully!");

    // 保持服务运行
    tokio::signal::ctrl_c().await?;
    info!("Received shutdown signal, stopping Bridge Service...");

    Ok(())
}

// 加载配置
async fn load_config() -> Result<BridgeConfig> {
    // 从环境变量或配置文件加载
    let mut config = BridgeConfig::default();

    if let Ok(udp_addr) = std::env::var("BRIDGE_UDP_BIND_ADDRESS") {
        config.udp_bind_address = udp_addr;
    }

    if let Ok(echokit_url) = std::env::var("ECHOKIT_WEBSOCKET_URL") {
        config.echokit_websocket_url = echokit_url;
    }

    if let Ok(api_url) = std::env::var("API_GATEWAY_WEBSOCKET_URL") {
        config.api_gateway_websocket_url = api_url;
    }

    if let Ok(max_sessions) = std::env::var("MAX_SESSIONS") {
        config.max_sessions = max_sessions.parse()
            .with_context(|| "Invalid MAX_SESSIONS value")?;
    }

    if let Ok(timeout) = std::env::var("SESSION_TIMEOUT_SECONDS") {
        config.session_timeout_seconds = timeout.parse()
            .with_context(|| "Invalid SESSION_TIMEOUT_SECONDS value")?;
    }

    if let Ok(mqtt_host) = std::env::var("MQTT_BROKER_HOST") {
        config.mqtt_broker_host = mqtt_host;
    }

    if let Ok(mqtt_port) = std::env::var("MQTT_BROKER_PORT") {
        config.mqtt_broker_port = mqtt_port.parse()
            .with_context(|| "Invalid MQTT_BROKER_PORT value")?;
    }

    Ok(config)
}

impl BridgeService {
    // 启动 Bridge 服务
    async fn start(
        &self,
        audio_output_rx: mpsc::UnboundedReceiver<(String, Vec<u8>)>,
    ) -> Result<()> {
        // MQTT 客户端已在 main 中启动

        // 启动 EchoKit 连接管理器
        self.echokit_manager.start().await
            .with_context(|| "Failed to start EchoKit connection manager")?;

        // 启动 UDP 服务器
        self.udp_server.start().await
            .with_context(|| "Failed to start UDP server")?;

        // 启动音频输出处理器
        self.start_audio_output_handler(audio_output_rx).await?;

        // 启动会话超时检查
        self.start_session_timeout_check().await?;

        // 启动心跳监控
        let heartbeat_monitor = self.heartbeat_monitor.clone();
        tokio::spawn(async move {
            heartbeat_monitor.start().await;
        });

        // 启动流控管理器
        let flow_controller = self.flow_controller.clone();
        tokio::spawn(async move {
            flow_controller.start().await;
        });

        // 启动健康检查服务
        self.start_health_check_service().await?;

        info!("All Bridge Service components started successfully");
        Ok(())
    }

    // 启动音频输出处理器
    async fn start_audio_output_handler(&self, mut audio_output_rx: mpsc::UnboundedReceiver<(String, Vec<u8>)>) -> Result<()> {
        let udp_server = self.udp_server.clone();

        tokio::spawn(async move {
            while let Some((device_id, audio_data)) = audio_output_rx.recv().await {
                if let Err(e) = udp_server.send_to_device(&device_id, audio_data).await {
                    error!("Failed to send audio output to device {}: {}", device_id, e);
                }
            }
        });

        Ok(())
    }

    // 启动会话超时检查
    async fn start_session_timeout_check(&self) -> Result<()> {
        let active_sessions = self.active_sessions.clone();
        let audio_processor = self.audio_processor.clone();
        let timeout_seconds = self.config.session_timeout_seconds;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));

            loop {
                interval.tick().await;

                let now = now_utc();
                let mut sessions_to_end = Vec::new();

                {
                    let sessions = active_sessions.read().await;
                    for (session_id, session_info) in sessions.iter() {
                        let duration = now.signed_duration_since(session_info.last_activity);
                        if duration.num_seconds() > timeout_seconds {
                            sessions_to_end.push(session_id.clone());
                        }
                    }
                }

                // 结束超时的会话
                for session_id in sessions_to_end {
                    warn!("Ending session {} due to timeout", session_id);
                    if let Err(e) = Self::end_session_internal(
                        active_sessions.clone(),
                        audio_processor.clone(),
                        &session_id,
                        "timeout"
                    ).await {
                        error!("Failed to end timeout session {}: {}", session_id, e);
                    }
                }
            }
        });

        Ok(())
    }

    // 启动健康检查服务
    async fn start_health_check_service(&self) -> Result<()> {
        let bind_address = "0.0.0.0:8082".to_string(); // 健康检查端口
        let ws_bind_address = "0.0.0.0:10031".to_string(); // WebSocket 端口
        let echokit_manager = self.echokit_manager.clone();
        let udp_server = self.udp_server.clone();
        let active_sessions = self.active_sessions.clone();
        let audio_processor = self.audio_processor.clone();
        let connection_manager = self.connection_manager.clone();
        let session_manager = self.session_manager.clone();
        let echokit_adapter = self.echokit_adapter.clone();

        // 启动健康检查 HTTP 服务
        tokio::spawn(async move {
            use axum::{
                extract::State,
                response::Json,
                routing::get,
                Router,
            };

            let app = Router::new()
                .route("/health", get(health_check))
                .route("/stats", get(get_stats))
                .with_state(AppState {
                    echokit_manager,
                    udp_server,
                    active_sessions,
                    audio_processor,
                });

            info!("Health check service listening on: {}", bind_address);

            let listener = tokio::net::TcpListener::bind(&bind_address).await.unwrap();
            if let Err(e) = axum::serve(listener, app).await {
                error!("Health check service error: {}", e);
            }
        });

        // 启动 WebSocket 服务器
        tokio::spawn(async move {
            use axum::{
                extract::State,
                routing::get,
                Router,
            };

            let ws_state = websocket::audio_handler::AppState {
                connection_manager,
                session_manager,
                echokit_adapter,
            };

            let app = Router::new()
                .route("/ws/audio", get(websocket::audio_handler::websocket_handler))
                .with_state(ws_state);

            info!("WebSocket server listening on: {}", ws_bind_address);

            let listener = tokio::net::TcpListener::bind(&ws_bind_address).await.unwrap();
            if let Err(e) = axum::serve(listener, app).await {
                error!("WebSocket server error: {}", e);
            }
        });

        Ok(())
    }

    // 内部方法：结束会话
    async fn end_session_internal(
        active_sessions: Arc<RwLock<std::collections::HashMap<String, SessionInfo>>>,
        audio_processor: Arc<audio_processor::AudioProcessor>,
        session_id: &str,
        reason: &str,
    ) -> Result<()> {
        let device_id = {
            let sessions = active_sessions.read().await;
            sessions.get(session_id).map(|s| s.device_id.clone())
        };

        if let Some(device_id) = device_id {
            // 结束音频处理会话
            if let Err(e) = audio_processor.end_session(&device_id, reason).await {
                error!("Failed to end audio session for device {}: {}", device_id, e);
            }

            // 从活跃会话中移除
            active_sessions.write().await.remove(session_id);

            info!("Ended session {} for device {} (reason: {})", session_id, device_id, reason);
        }

        Ok(())
    }
}

// 应用状态（用于健康检查服务）
#[derive(Clone)]
struct AppState {
    echokit_manager: Arc<echokit_client::EchoKitConnectionManager>,
    udp_server: Arc<udp_server::UdpAudioServer>,
    active_sessions: Arc<RwLock<std::collections::HashMap<String, SessionInfo>>>,
    audio_processor: Arc<audio_processor::AudioProcessor>,
}

// 健康检查端点
async fn health_check(State(state): State<AppState>) -> Json<serde_json::Value> {
    let echokit_connected = state.echokit_manager.get_client().is_connected().await;
    let active_sessions = state.active_sessions.read().await.len();

    // 修改健康检查逻辑：只要服务启动就认为是健康的，不依赖外部 EchoKit Server
    Json(serde_json::json!({
        "status": "healthy",
        "service": "echo-bridge",
        "echokit_connected": echokit_connected,
        "active_sessions": active_sessions,
        "timestamp": now_utc()
    }))
}

// 统计信息端点
async fn get_stats(State(state): State<AppState>) -> Json<BridgeServiceStats> {
    let echokit_client = state.echokit_manager.get_client();
    let echokit_connected = echokit_client.is_connected().await;
    let echokit_sessions = echokit_client.get_active_sessions_count().await;
    let active_sessions = state.active_sessions.read().await.len();
    let audio_sessions = state.audio_processor.get_active_sessions_count().await;
    let udp_stats = state.udp_server.get_stats().await;

    Json(BridgeServiceStats {
        echokit_connected,
        echokit_sessions,
        bridge_sessions: active_sessions,
        audio_sessions,
        online_devices: udp_stats.online_devices,
        uptime_seconds: 0,
    })
}

// Bridge 服务统计信息
#[derive(serde::Serialize)]
struct BridgeServiceStats {
    echokit_connected: bool,
    echokit_sessions: usize,
    bridge_sessions: usize,
    audio_sessions: usize,
    online_devices: usize,
    uptime_seconds: u64,
}