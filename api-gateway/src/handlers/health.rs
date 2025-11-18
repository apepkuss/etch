use axum::{extract::State, response::Json, routing::get, Router};
use echo_shared::ApiResponse;
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::app_state::AppState;

pub async fn health_check() -> Json<ApiResponse<serde_json::Value>> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let health_data = json!({
        "status": "healthy",
        "timestamp": timestamp,
        "service": "echo-api-gateway",
        "version": "0.1.0"
    });

    Json(ApiResponse::success(health_data))
}

pub async fn detailed_health_check(
    State(app_state): State<AppState>,
) -> Json<ApiResponse<serde_json::Value>> {
    let system_info = app_state.get_system_info().await;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let health_data = json!({
        "status": system_info.status.health,
        "timestamp": timestamp,
        "service": "echo-api-gateway",
        "version": system_info.status.version,
        "environment": system_info.status.environment,
        "start_time": system_info.status.start_time,
        "uptime_seconds": system_info.runtime.uptime_seconds,
        "statistics": {
            "total_requests": system_info.stats.total_requests,
            "active_connections": system_info.stats.active_connections,
            "authenticated_users": system_info.stats.authenticated_users,
            "device_count": system_info.stats.device_count,
            "session_count": system_info.stats.session_count,
            "errors": system_info.stats.errors,
            "last_updated": system_info.stats.last_updated
        },
        "runtime": {
            "memory_usage_mb": system_info.runtime.memory_usage_mb,
            "cpu_usage_percent": system_info.runtime.cpu_usage_percent
        },
        "features": {
            "auth_enabled": system_info.config.features.auth_enabled,
            "websocket_enabled": system_info.config.features.websocket_enabled,
            "sessions_enabled": system_info.config.features.sessions_enabled,
            "rate_limiting": system_info.config.features.rate_limiting
        },
        "dependencies": {
            "database": "offline", // TODO: 实际检查数据库连接
            "redis": "offline",    // TODO: 实际检查Redis连接
            "mqtt": "offline"      // TODO: 实际检查MQTT连接
        }
    });

    Json(ApiResponse::success(health_data))
}

pub fn health_routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/", get(health_check))
        .route("/basic", get(health_check))
        .route("/detailed", get(detailed_health_check))
}