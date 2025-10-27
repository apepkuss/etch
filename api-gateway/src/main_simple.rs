use axum::{
    routing::get,
    Router,
};
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};
use tracing::{info, Level};
use tracing_subscriber;

mod handlers_simple;
mod websocket;

#[tokio::main]
async fn main() {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_target(false)
        .init();

    info!("Echo API Gateway starting...");

    // 创建应用
    let app = create_app();

    // 启动服务器
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    info!("API Gateway listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

fn create_app() -> Router {
    Router::new()
        // 健康检查路由
        .route("/health", get(handlers_simple::health_check))
        .route("/api/v1/health", get(handlers_simple::health_check))

        // 设备管理路由
        .route("/api/v1/devices", get(handlers_simple::get_devices))
        .route("/api/v1/devices/stats", get(handlers_simple::get_device_stats))

        // 会话管理路由
        .route("/api/v1/sessions", get(handlers_simple::get_sessions))
        .route("/api/v1/sessions/stats", get(handlers_simple::get_session_stats))

        // 认证路由
        .route("/api/v1/auth/login", get(handlers_simple::login))

        // WebSocket 路由
        .route("/ws", get(websocket::websocket_handler))

        // CORS 中间件
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any)
        )
}