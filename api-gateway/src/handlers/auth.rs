use axum::{
    extract::{State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use echo_shared::{AppConfig, ApiResponse, UserRole, User};
use serde_json::json;
use serde::{Deserialize, Serialize};
use crate::app_state::AppState;

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

// 数据库登录处理
pub async fn login(
    State(app_state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<ApiResponse<LoginResponse>>, StatusCode> {
    // 使用数据库验证用户凭据
    match app_state.user_service.verify_password(&payload.username, &payload.password).await {
        Ok(Some(user)) => {
            // 生成 JWT token
            let token = echo_shared::generate_jwt(
                &user.id,
                &user.username,
                user.role.clone(),
                "your-super-secret-jwt-key-change-in-production", // 从环境变量或配置获取
                24, // 24小时过期
            ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            let login_response = LoginResponse {
                token,
                user: UserInfo {
                    id: user.id,
                    username: user.username,
                    email: user.email,
                    role: user.role,
                },
                expires_in: 24 * 3600, // 24小时
            };

            Ok(Json(ApiResponse::success(login_response)))
        }
        Ok(None) => {
            Ok(Json(ApiResponse::error("Invalid username or password".to_string())))
        }
        Err(_) => {
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
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

pub fn auth_routes() -> Router<AppState> {
    Router::new()
        .route("/login", post(login))
        .route("/me", get(get_user_info))
        .route("/logout", post(logout))
}