use anyhow::Result;
use axum::{
    middleware,
    routing::{get, post, put, delete},
    Router,
};
use echo_shared::{load_config, AppConfig};
use std::net::SocketAddr;
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::{info, Level};
use tracing_subscriber;

mod auth;
mod handlers;
mod middleware;
mod models;
mod utils;
mod websocket;
mod mqtt;

use handlers::{auth::auth_routes, devices::device_routes, sessions::session_routes, health::health_routes};
use middleware::{auth_middleware, request_logging};

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

    // 构建应用
    let app = create_app(config.clone()).await?;

    // 启动服务器
    let addr = SocketAddr::from(([0, 0, 0, 0], config.server.port));
    info!("API Gateway listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn create_app(config: AppConfig) -> Result<Router> {
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

        // WebSocket 路由
        .route("/ws", get(websocket::websocket_handler))

        // 应用中间件
        .layer(middleware_layer)

        // 添加状态
        .with_state(config);

    Ok(app)
}