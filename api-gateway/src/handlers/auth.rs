use axum::{
    extract::{State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use echo_shared::{ApiResponse, UserRole};
use serde_json::json;
use serde::{Deserialize, Serialize};
use crate::app_state::AppState;
use jsonwebtoken::{encode, Header, EncodingKey};
use chrono::{Duration, Utc};

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

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,     // 用户ID
    pub username: String,
    pub role: UserRole,
    pub exp: i64,        // 过期时间
    pub iat: i64,        // 签发时间
}

// 简化的登录处理（硬编码验证，后续可连接数据库）
pub async fn login(
    State(_app_state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<ApiResponse<LoginResponse>>, StatusCode> {
    // 简化的用户验证（硬编码，仅用于测试）
    if payload.username == "admin" && payload.password == "admin123" {
        let user_info = UserInfo {
            id: "admin-001".to_string(),
            username: "admin".to_string(),
            email: "admin@echo.system".to_string(),
            role: UserRole::Admin,
        };

        // 生成 JWT token
        let token = generate_jwt_token(&user_info).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let login_response = LoginResponse {
            token,
            user: user_info,
            expires_in: 24 * 3600, // 24小时
        };

        Ok(Json(ApiResponse::success(login_response)))
    } else if payload.username == "user" && payload.password == "user123" {
        let user_info = UserInfo {
            id: "user-001".to_string(),
            username: "user".to_string(),
            email: "user@echo.system".to_string(),
            role: UserRole::User,
        };

        // 生成 JWT token
        let token = generate_jwt_token(&user_info).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let login_response = LoginResponse {
            token,
            user: user_info,
            expires_in: 24 * 3600, // 24小时
        };

        Ok(Json(ApiResponse::success(login_response)))
    } else {
        Ok(Json(ApiResponse::error("Invalid username or password".to_string())))
    }
}

// 生成JWT token
fn generate_jwt_token(user: &UserInfo) -> Result<String, Box<dyn std::error::Error>> {
    let now = Utc::now();
    let exp = now + Duration::hours(24);

    let claims = Claims {
        sub: user.id.clone(),
        username: user.username.clone(),
        role: user.role.clone(),
        exp: exp.timestamp(),
        iat: now.timestamp(),
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret("your-super-secret-jwt-key-change-in-production".as_ref()),
    )?;

    Ok(token)
}

// 用户信息获取（简化版，实际应从JWT解析）
pub async fn get_user_info(
    State(_app_state): State<AppState>,
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