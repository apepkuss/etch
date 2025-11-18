use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use tracing::{info, warn, error};
use std::time::{Duration, Instant};
use crate::app_state::AppState;

pub async fn request_logging(
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let start = Instant::now();
    let method = req.method().clone();
    let uri = req.uri().clone();

    info!("Incoming request: {} {}", method, uri);

    let response = next.run(req).await;
    let status = response.status();
    let duration = start.elapsed();

    // TODO: 在实际应用中更新统计信息
    // 由于Axum中间件签名限制，暂时跳过统计更新

    if status.is_success() {
        info!("Request completed: {} {} - {}ms", method, uri, duration.as_millis());
    } else if status.is_client_error() {
        warn!("Client error: {} {} - {} ({})", method, uri, status, duration.as_millis());
    } else {
        error!("Server error: {} {} - {} ({})", method, uri, status, duration.as_millis());
    }

    Ok(response)
}

pub async fn auth_middleware(
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // 检查 Authorization header
    let auth_header = req
        .headers()
        .get("authorization")
        .and_then(|h| h.to_str().ok());

    if let Some(auth_header) = auth_header {
        if auth_header.starts_with("Bearer ") {
            let token = &auth_header[7..];
            // 这里应该验证 JWT token
            // 暂时跳过实际验证，后续在 auth 模块中实现
            return Ok(next.run(req).await);
        }
    }

    // 对于不需要认证的路径（如健康检查、登录、设备注册等），直接通过
    let path = req.uri().path();
    if path == "/health"
        || path.starts_with("/health")
        || path.starts_with("/auth")
        || path.starts_with("/api/v1/auth")
        || path.starts_with("/api/v1/devices/register")
        || path.starts_with("/api/v1/devices/verify")
        || path == "/ws" {
        return Ok(next.run(req).await);
    }

    Err(StatusCode::UNAUTHORIZED)
}

pub async fn rate_limit_middleware(
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // 简单的速率限制实现
    // 实际生产环境中应该使用更复杂的限流算法
    let client_ip = req
        .headers()
        .get("x-forwarded-for")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("unknown");

    // TODO: 实现 Redis 基础的速率限制
    info!("Rate limiting check for IP: {}", client_ip);

    Ok(next.run(req).await)
}