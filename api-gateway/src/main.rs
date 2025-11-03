use anyhow::Result;
use axum::{
    routing::get,
    Router,
};
use echo_shared::{AppConfig};
use std::net::SocketAddr;
use tower_http::{
    cors::{Any, CorsLayer},
};
use tracing::{info, Level};
use tracing_subscriber;
use tokio::sync::broadcast;
use serde_json::json;
use chrono;

// 逐步重新启用模块
// mod auth;
mod handlers;
mod middleware;
// mod models;
// mod utils;
mod websocket;
// mod mqtt;
// mod storage;
mod database;
mod cache;
// mod device_service;
// mod user_service;
mod app_state;

// 启用基础的handlers
use handlers::health::health_routes;
use handlers::auth::auth_routes;
use handlers::devices::device_routes;
use handlers::users::user_routes;
use handlers::sessions::session_routes;
use app_state::AppState;
use middleware::{auth_middleware, request_logging};
use websocket::websocket_handler;
// use mqtt::{ApiGatewayMqttClient, mqtt_routes};
// use storage::{Storage, StorageConfig};
// use device_service::DeviceService;
// use user_service::UserService;
// use app_state::AppState;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_target(false)
        .init();

    // 创建简化的配置（暂时跳过复杂的模块）
    let config = AppConfig {
        server: echo_shared::ServerConfig {
            host: "0.0.0.0".to_string(),
            port: 8080,
            workers: 4,
        },
        database: echo_shared::DatabaseConfig {
            url: "postgres://echo_user:echo_password@localhost:5432/echo_db".to_string(),
            max_connections: 10,
            min_connections: 1,
        },
        redis: echo_shared::RedisConfig {
            url: "redis://:redis_password@localhost:6379".to_string(),
            max_connections: 10,
        },
        jwt: echo_shared::JwtConfig {
            secret: "your-super-secret-jwt-key-change-in-production".to_string(),
            expiration_hours: 24,
        },
        mqtt: echo_shared::types::MqttConfig {
            broker: "localhost".to_string(),
            port: 1883,
            username: None,
            password: None,
        },
    };
    info!("Configuration loaded successfully");

    // TODO: 临时禁用存储层和MQTT以修复编译问题
    // 初始化存储层
    // let storage_config = StorageConfig::default();
    // info!("Initializing storage layer...");
    // let storage = Arc::new(Storage::new(storage_config).await?);
    // info!("Storage layer initialized successfully");

    // 创建 WebSocket 广播器（简化版，虽然未使用但保留用于将来扩展）
    let (_websocket_tx, _websocket_rx) = broadcast::channel::<echo_shared::WebSocketMessage>(1000);

    // TODO: 临时禁用 MQTT 客户端
    // 创建 MQTT 配置
    // let mqtt_config = MqttConfig {
    //     broker_host: std::env::var("MQTT_BROKER_HOST")
    //         .unwrap_or_else(|_| "localhost".to_string()),
    //     broker_port: std::env::var("MQTT_BROKER_PORT")
    //         .unwrap_or_else(|_| "1883".to_string())
    //         .parse()
    //         .unwrap_or(1883),
    //     client_id: format!("api-gateway-{}", uuid::Uuid::new_v4()),
    //     username: std::env::var("MQTT_USERNAME").ok(),
    //     password: std::env::var("MQTT_PASSWORD").ok(),
    //     keep_alive: 60,
    //     clean_session: true,
    //     max_reconnect_attempts: 10,
    //     reconnect_interval_ms: 5000,
    // };

    // 创建 MQTT 客户端
    // let mqtt_client = Arc::new(ApiGatewayMqttClient::new(
    //     mqtt_config.clone(),
    //     websocket_tx,
    // )?);

    // 启动 MQTT 客户端
    // info!("Starting MQTT client...");
    // mqtt_client.start().await?;

    // 订阅主题
    // mqtt_client.subscribe(&TopicFilter::all_device_status()).await?;
    // mqtt_client.subscribe(&TopicFilter::all_device_wake()).await?;
    // mqtt_client.subscribe(&TopicFilter::system_status()).await?;

    // info!("MQTT client started and subscribed to topics");

    // TODO: 临时禁用服务层
    // 创建服务层
    // let device_service = Arc::new(DeviceService::new(storage.db.clone(), storage.cache.clone()));
    // let user_service = Arc::new(UserService::new(storage.db.clone(), storage.cache.clone()));

    // 暂时跳过完整的应用状态创建，直接使用简化的路由
    // TODO: 实现完整的应用状态初始化

    // 创建应用（使用真正的handlers和AppState）
    let app_state = AppState::new().await?;

    // 创建 API v1 路由组合
    let api_v1_routes = Router::new()
        .nest("/auth", auth_routes())
        .nest("/devices", device_routes())
        .nest("/users", user_routes())
        .nest("/sessions", session_routes());

    let app = Router::new()
        // 健康检查路由（无需认证）
        .route("/health", get(handlers::health::health_check))
        .nest("/health", health_routes())

        // WebSocket 路由
        .route("/ws", get(websocket_handler))

        // API v1 路由（需要认证）
        .nest("/api/v1", api_v1_routes)

        .with_state(app_state)
        .layer(axum::middleware::from_fn(request_logging))
        .layer(axum::middleware::from_fn(auth_middleware))
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any));

    // 启动服务器
    let addr = SocketAddr::from(([0, 0, 0, 0], config.server.port));
    info!("API Gateway listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

// 简单的健康检查端点
async fn health_check_simple() -> axum::response::Json<serde_json::Value> {
    axum::response::Json(json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "service": "echo-api-gateway",
        "version": "0.1.0-simplified"
    }))
}

// 注释掉复杂的 app 创建函数
/*
async fn create_app(config: AppConfig, app_state: Arc<AppState>) -> Result<Router> {
    // 创建中间件层
    let middleware_layer = ServiceBuilder::new()
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
        .layer(axum_middleware::from_fn(request_logging))
        .into_inner();

    // 创建 API v1 路由组合
    let api_v1_routes = Router::new()
        .nest("/auth", auth_routes())
        .nest("/health", health_routes())
        .nest("/devices", device_routes())
        .nest("/sessions", session_routes())
        .nest("/mqtt", mqtt_routes());

    // 创建主路由
    let app = Router::new()
        // 健康检查路由（无需认证）
        .route("/health", get(handlers::health::health_check))

        // API v1 路由（需要认证）
        .nest("/api/v1", api_v1_routes)

        // WebSocket 路由
        .route("/ws", get(websocket::websocket_handler))

        // 应用中间件
        .layer(middleware_layer)

        // 添加状态
        .with_state(app_state.as_ref().clone());

    Ok(app)
}
*/