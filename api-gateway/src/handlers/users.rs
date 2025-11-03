use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use echo_shared::{ApiResponse, User, UserRole, PaginationParams, PaginatedResponse, generate_uuid};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use crate::app_state::AppState;
use bcrypt::{hash, verify, DEFAULT_COST};

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub email: String,
    pub password: String,
    pub role: Option<UserRole>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
    pub username: Option<String>,
    pub email: Option<String>,
    pub password: Option<String>,
    pub role: Option<UserRole>,
}

#[derive(Debug, Deserialize)]
pub struct UserQueryParams {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub role: Option<UserRole>,
    pub username: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

// 模拟用户数据存储
static mut USERS: Option<HashMap<String, User>> = None;

fn get_mock_users() -> &'static mut HashMap<String, User> {
    unsafe {
        if USERS.is_none() {
            let mut users = HashMap::new();

            // 创建默认管理员用户
            let admin_password_hash = hash("admin123", DEFAULT_COST).unwrap_or_else(|_| "hashed".to_string());
            let user_password_hash = hash("user123", DEFAULT_COST).unwrap_or_else(|_| "hashed".to_string());

            users.insert("admin-001".to_string(), User {
                id: "admin-001".to_string(),
                username: "admin".to_string(),
                email: "admin@echo.system".to_string(),
                password_hash: admin_password_hash,
                role: UserRole::Admin,
            });

            users.insert("user-001".to_string(), User {
                id: "user-001".to_string(),
                username: "user".to_string(),
                email: "user@echo.system".to_string(),
                password_hash: user_password_hash,
                role: UserRole::User,
            });

            USERS = Some(users);
        }
        USERS.as_mut().unwrap()
    }
}

// 获取用户列表
pub async fn get_users(
    State(_app_state): State<AppState>,
    Query(params): Query<UserQueryParams>,
) -> Json<ApiResponse<PaginatedResponse<User>>> {
    let pagination = PaginationParams {
        page: params.page.unwrap_or(1),
        page_size: params.page_size.unwrap_or(20),
    };

    let users = get_mock_users();
    let mut user_list: Vec<User> = users.values().cloned().collect();

    // 应用过滤条件
    if let Some(role) = params.role {
        user_list.retain(|u| u.role == role);
    }

    if let Some(username) = params.username {
        user_list.retain(|u| u.username.to_lowercase().contains(&username.to_lowercase()));
    }

    if let Some(email) = params.email {
        user_list.retain(|u| u.email.to_lowercase().contains(&email.to_lowercase()));
    }

    // 只返回不包含密码哈希的用户信息
    let safe_users: Vec<User> = user_list.into_iter().map(|mut u| {
        u.password_hash = "***".to_string(); // 隐藏密码哈希
        u
    }).collect();

    // 按用户ID排序（作为创建时间的替代）
    let mut sorted_users = safe_users;
    sorted_users.sort_by(|a, b| a.id.cmp(&b.id));

    // 应用分页
    let total = sorted_users.len() as u64;
    let offset = echo_shared::calculate_offset(pagination.page, pagination.page_size) as usize;
    let end = (offset + pagination.page_size as usize).min(sorted_users.len());

    let paginated_users = if offset < sorted_users.len() {
        sorted_users[offset..end].to_vec()
    } else {
        vec![]
    };

    let response = PaginatedResponse::new(paginated_users, total, pagination);
    Json(ApiResponse::success(response))
}

// 获取单个用户详情
pub async fn get_user(
    Path(user_id): Path<String>,
    State(_app_state): State<AppState>,
) -> Result<Json<ApiResponse<User>>, StatusCode> {
    let users = get_mock_users();

    if let Some(mut user) = users.get(&user_id).cloned() {
        // 隐藏密码哈希
        user.password_hash = "***".to_string();
        Ok(Json(ApiResponse::success(user)))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

// 创建新用户
pub async fn create_user(
    State(_app_state): State<AppState>,
    Json(payload): Json<CreateUserRequest>,
) -> Result<Json<ApiResponse<User>>, (StatusCode, Json<ApiResponse<()>>)> {
    // 验证输入
    if payload.username.is_empty() || payload.email.is_empty() || payload.password.is_empty() {
        let response = ApiResponse::error("Username, email, and password are required".to_string());
        return Err((StatusCode::BAD_REQUEST, Json(response)));
    }

    // 检查用户名是否已存在
    let users = get_mock_users();
    if users.values().any(|u| u.username == payload.username) {
        let response = ApiResponse::error("Username already exists".to_string());
        return Err((StatusCode::CONFLICT, Json(response)));
    }

    // 检查邮箱是否已存在
    if users.values().any(|u| u.email == payload.email) {
        let response = ApiResponse::error("Email already exists".to_string());
        return Err((StatusCode::CONFLICT, Json(response)));
    }

    // 密码加密
    let password_hash = hash(&payload.password, DEFAULT_COST)
        .map_err(|_| {
            let response = ApiResponse::error("Failed to hash password".to_string());
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        })?;

    // 创建新用户
    let new_user = User {
        id: generate_uuid(),
        username: payload.username.clone(),
        email: payload.email.clone(),
        password_hash,
        role: payload.role.unwrap_or(UserRole::User),
    };

    // 存储用户
    users.insert(new_user.id.clone(), new_user.clone());

    // 返回不包含密码哈希的用户信息
    let mut safe_user = new_user.clone();
    safe_user.password_hash = "***".to_string();

    Ok(Json(ApiResponse::success(safe_user)))
}

// 更新用户信息
pub async fn update_user(
    Path(user_id): Path<String>,
    State(_app_state): State<AppState>,
    Json(payload): Json<UpdateUserRequest>,
) -> Result<Json<ApiResponse<User>>, (StatusCode, Json<ApiResponse<()>>)> {
    let users = get_mock_users();

    // 首先检查用户是否存在
    let existing_user = users.get(&user_id).cloned();
    if existing_user.is_none() {
        let response = ApiResponse::error("User not found".to_string());
        return Err((StatusCode::NOT_FOUND, Json(response)));
    }

    let existing_user = existing_user.unwrap();

    // 检查用户名冲突（需要排除当前用户）
    if let Some(new_username) = &payload.username {
        if new_username != &existing_user.username {
            if users.values().any(|u| u.id != user_id && u.username == *new_username) {
                let response = ApiResponse::error("Username already exists".to_string());
                return Err((StatusCode::CONFLICT, Json(response)));
            }
        }
    }

    // 检查邮箱冲突（需要排除当前用户）
    if let Some(new_email) = &payload.email {
        if new_email != &existing_user.email {
            if users.values().any(|u| u.id != user_id && u.email == *new_email) {
                let response = ApiResponse::error("Email already exists".to_string());
                return Err((StatusCode::CONFLICT, Json(response)));
            }
        }
    }

    // 现在可以安全地更新用户
    if let Some(user) = users.get_mut(&user_id) {
        // 更新用户名
        if let Some(new_username) = &payload.username {
            user.username = new_username.clone();
        }

        // 更新邮箱
        if let Some(new_email) = &payload.email {
            user.email = new_email.clone();
        }

        // 更新密码（如果提供）
        if let Some(new_password) = &payload.password {
            if !new_password.is_empty() {
                user.password_hash = hash(new_password, DEFAULT_COST)
                    .map_err(|_| {
                        let response = ApiResponse::error("Failed to hash password".to_string());
                        (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
                    })?;
            }
        }

        // 更新角色
        if let Some(new_role) = &payload.role {
            user.role = new_role.clone();
        }

        // Note: User struct doesn't have updated_at field in shared types

        // 返回不包含密码哈希的用户信息
        let mut safe_user = user.clone();
        safe_user.password_hash = "***".to_string();

        Ok(Json(ApiResponse::success(safe_user)))
    } else {
        let response = ApiResponse::error("User not found".to_string());
        Err((StatusCode::NOT_FOUND, Json(response)))
    }
}

// 删除用户
pub async fn delete_user(
    Path(user_id): Path<String>,
    State(_app_state): State<AppState>,
) -> Json<ApiResponse<serde_json::Value>> {
    let users = get_mock_users();

    if users.remove(&user_id).is_some() {
        let response = json!({
            "message": "User deleted successfully",
            "user_id": user_id
        });
        Json(ApiResponse::success(response))
    } else {
        Json(ApiResponse::error("User not found".to_string()))
    }
}

// 修改密码
pub async fn change_password(
    Path(user_id): Path<String>,
    State(_app_state): State<AppState>,
    Json(payload): Json<ChangePasswordRequest>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    let users = get_mock_users();

    if let Some(user) = users.get_mut(&user_id) {
        // 验证当前密码
        if verify(&payload.current_password, &user.password_hash).unwrap_or(false) {
            // 设置新密码
            user.password_hash = hash(&payload.new_password, DEFAULT_COST)
                .map_err(|_| {
                    let response = ApiResponse::error("Failed to hash new password".to_string());
                    (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
                })?;

            // Note: User struct doesn't have updated_at field in shared types

            Ok(Json(ApiResponse::success(())))
        } else {
            let response = ApiResponse::error("Current password is incorrect".to_string());
            Err((StatusCode::UNAUTHORIZED, Json(response)))
        }
    } else {
        let response = ApiResponse::error("User not found".to_string());
        Err((StatusCode::NOT_FOUND, Json(response)))
    }
}

// 获取用户统计信息
pub async fn get_user_stats(
    State(_app_state): State<AppState>,
) -> Json<ApiResponse<serde_json::Value>> {
    let users = get_mock_users();

    let total = users.len();
    let admin = users.values().filter(|u| u.role == UserRole::Admin).count();
    let user_role = users.values().filter(|u| u.role == UserRole::User).count();

    // 简化的统计信息，因为没有 created_at 和 is_active 字段
    let stats = json!({
        "total": total,
        "by_role": {
            "admin": admin,
            "user": user_role
        },
        "note": "Detailed statistics require timestamp fields in User struct"
    });

    Json(ApiResponse::success(stats))
}

pub fn user_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(get_users).post(create_user))
        .route("/stats", get(get_user_stats))
        .route("/:id", get(get_user))
        .route("/:id", post(update_user))
        .route("/:id", axum::routing::delete(delete_user))
        .route("/:id/change-password", post(change_password))
}