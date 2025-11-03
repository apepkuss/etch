use axum::{
    extract::{Request, State},
    http::{header::AUTHORIZATION, StatusCode},
    middleware::Next,
    response::Response,
};
use echo_shared::{verify_jwt, Claims, UserRole, AppConfig, EchoError};

pub trait AuthExt {
    fn auth_required() -> Self;
}

impl AuthExt for axum::middleware::FromFnMiddleware<impl Fn(Request, Next) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, StatusCode>> + Send>> + Clone> {
    fn auth_required() -> Self {
        axum::middleware::from_fn(auth_middleware)
    }
}

async fn auth_middleware(
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // 检查 Authorization header
    let auth_header = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    if let Some(auth_header) = auth_header {
        if auth_header.starts_with("Bearer ") {
            let token = &auth_header[7..];

            // TODO: 从状态中获取配置并验证 JWT
            // let config = req.extract::<State<AppConfig>>().await?;

            // 暂时跳过 JWT 验证，后续实现
            return Ok(next.run(req).await);
        }
    }

    // 对于不需要认证的路径，直接通过
    let path = req.uri().path();
    if path == "/health" || path == "/api/v1/health" || path == "/api/v1/auth/login" || path.starts_with("/ws") {
        return Ok(next.run(req).await);
    }

    Err(StatusCode::UNAUTHORIZED)
}

// 权限检查中间件
pub async fn require_role(required_role: UserRole) -> impl Fn(Request, Next) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, StatusCode>> + Send>> {
    move |req: Request, next: Next| {
        Box::pin(async move {
            // TODO: 从 JWT token 中提取用户角色并检查权限
            // 暂时允许所有请求通过
            Ok(next.run(req).await)
        })
    }
}