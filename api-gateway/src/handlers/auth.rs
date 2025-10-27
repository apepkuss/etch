use axum::{
    extract::{State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use echo_shared::{AppConfig, ApiResponse, UserRole};
use serde_json::json;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: UserInfo,
    pub expires_in: u64,
}

#[derive(Debug, Serialize)]
pub struct UserInfo {
    pub id: String,
    pub username: String,
    pub email: String,
    pub role: UserRole,
}

// 模拟登录处理
pub async fn login(
    State(config): State<AppConfig>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<ApiResponse<LoginResponse>>, StatusCode> {
    // TODO: 实现实际的用户认证逻辑
    // 暂时使用硬编码的管理员账户进行演示
    if payload.username == "admin" && payload.password == "admin123" {
        let user_id = "admin-001".to_string();
        let token = echo_shared::generate_jwt(
            &user_id,
            &payload.username,
            UserRole::Admin,
            &config.jwt.secret,
            config.jwt.expiration_hours,
        ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let login_response = LoginResponse {
            token,
            user: UserInfo {
                id: user_id,
                username: payload.username,
                email: "admin@echo.system".to_string(),
                role: UserRole::Admin,
            },
            expires_in: config.jwt.expiration_hours * 3600,
        };

        Ok(Json(ApiResponse::success(login_response)))
    } else {
        Ok(Json(ApiResponse::error("Invalid username or password".to_string())))
    }
}

// 用户信息获取
pub async fn get_user_info(
    State(config): State<AppConfig>,
) -> Result<Json<ApiResponse<UserInfo>>, StatusCode> {
    // TODO: 从 JWT token 中解析用户信息
    let user_info = UserInfo {
        id: "admin-001".to_string(),
        username: "admin".to_string(),
        email: "admin@echo.system".to_string(),
        role: UserRole::Admin,
    };

    Ok(Json(ApiResponse::success(user_info)))
}

// 退出登录
pub async fn logout() -> Json<ApiResponse<serde_json::Value>> {
    // TODO: 实现 token 黑名单机制
    let response = json!({
        "message": "Logged out successfully"
    });

    Json(ApiResponse::success(response))
}

pub fn auth_routes() -> Router<AppConfig> {
    Router::new()
        .route("/auth/login", post(login))
        .route("/auth/me", get(get_user_info))
        .route("/auth/logout", post(logout))
}