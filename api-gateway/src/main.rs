use anyhow::Result;
use axum::{
    middleware,
    routing::{get, post, put, delete},
    Router,
};
use echo_shared::{load_config, AppConfig, MqttConfig, TopicFilter, QoS};
use std::net::SocketAddr;
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::{info, Level};
use tracing_subscriber;
use tokio::sync::broadcast;

mod auth;
mod handlers;
mod middleware;
mod models;
mod utils;
mod websocket;
mod mqtt;
mod storage;
mod device_service;
mod user_service;
mod app_state;

use handlers::{auth::auth_routes, devices::device_routes, sessions::session_routes, health::health_routes};
use middleware::{auth_middleware, request_logging};
use mqtt::{ApiGatewayMqttClient, mqtt_routes};
use storage::{Storage, StorageConfig};
use device_service::DeviceService;
use user_service::UserService;
use app_state::AppState;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_target(false)
        .init();

    // 加载配置
    let config = load_config()?;
    info!("Configuration loaded successfully");

    // 初始化存储层
    let storage_config = StorageConfig::default();
    info!("Initializing storage layer...");
    let storage = Arc::new(Storage::new(storage_config).await?);
    info!("Storage layer initialized successfully");

    // 创建 WebSocket 广播器
    let (websocket_tx, _websocket_rx) = broadcast::channel(1000);

    // 创建 MQTT 配置
    let mqtt_config = MqttConfig {
        broker_host: std::env::var("MQTT_BROKER_HOST")
            .unwrap_or_else(|_| "localhost".to_string()),
        broker_port: std::env::var("MQTT_BROKER_PORT")
            .unwrap_or_else(|_| "1883".to_string())
            .parse()
            .unwrap_or(1883),
        client_id: format!("api-gateway-{}", uuid::Uuid::new_v4()),
        username: std::env::var("MQTT_USERNAME").ok(),
        password: std::env::var("MQTT_PASSWORD").ok(),
        keep_alive: 60,
        clean_session: true,
        max_reconnect_attempts: 10,
        reconnect_interval_ms: 5000,
    };

    // 创建 MQTT 客户端
    let mqtt_client = Arc::new(ApiGatewayMqttClient::new(
        mqtt_config.clone(),
        websocket_tx,
    )?);

    // 启动 MQTT 客户端
    info!("Starting MQTT client...");
    mqtt_client.start().await?;

    // 订阅主题
    mqtt_client.subscribe(&TopicFilter::all_device_status()).await?;
    mqtt_client.subscribe(&TopicFilter::all_device_wake()).await?;
    mqtt_client.subscribe(&TopicFilter::system_status()).await?;

    info!("MQTT client started and subscribed to topics");

    // 创建服务层
    let device_service = Arc::new(DeviceService::new(storage.db.clone(), storage.cache.clone()));
    let user_service = Arc::new(UserService::new(storage.db.clone(), storage.cache.clone()));

    // 创建应用状态
    let app_state = Arc::new(AppState::new(
        storage.clone(),
        device_service.clone(),
        user_service.clone(),
        mqtt_client.clone(),
        websocket_tx.clone(),
    ));

    // 构建应用
    let app = create_app(config.clone(), app_state.clone()).await?;

    // 启动服务器
    let addr = SocketAddr::from(([0, 0, 0, 0], config.server.port));
    info!("API Gateway listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn create_app(config: AppConfig, app_state: Arc<AppState>) -> Result<Router> {
    // 创建中间件层
    let middleware_layer = ServiceBuilder::new()
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
        .layer(middleware::from_fn(request_logging))
        .into_inner();

    // 创建路由
    let app = Router::new()
        // 健康检查路由（无需认证）
        .route("/health", get(handlers::health::health_check))
        .nest("/api/v1", health_routes())

        // API 路由（需要认证）
        .route("/api/v1/auth/login", post(handlers::auth::login))
        .nest("/api/v1", auth_routes())
        .nest("/api/v1", device_routes())
        .nest("/api/v1", session_routes())

        // MQTT 路由
        .nest("/api/v1", mqtt_routes())

        // WebSocket 路由
        .route("/ws", get(websocket::websocket_handler))

        // 应用中间件
        .layer(middleware_layer)

        // 添加状态
        .with_state(app_state);

    Ok(app)
}