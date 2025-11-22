use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post, delete},
    Router,
};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use tracing::{info, error};
use crate::app_state::AppState;
use echo_shared::ApiResponse;

/// EchoKit Server 数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EchoKitServer {
    pub id: i32,
    pub user_id: String,
    pub server_url: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 添加服务器请求
#[derive(Debug, Deserialize)]
pub struct AddServerRequest {
    pub server_url: String,
}


/// 获取用户的 EchoKit Server 列表
pub async fn get_servers(
    State(app_state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<EchoKitServer>>>, StatusCode> {
    // TODO: 从认证中间件获取真实的 user_id
    let user_id = "user001"; // 临时使用固定值

    match sqlx::query_as!(
        EchoKitServer,
        r#"
        SELECT
            id,
            user_id as "user_id!",
            server_url as "server_url!",
            created_at as "created_at!",
            updated_at as "updated_at!"
        FROM echokit_servers
        WHERE user_id = $1
        ORDER BY created_at DESC
        "#,
        user_id
    )
    .fetch_all(app_state.database.pool())
    .await
    {
        Ok(servers) => {
            info!("Retrieved {} EchoKit servers for user {}", servers.len(), user_id);
            Ok(Json(ApiResponse::success(servers)))
        }
        Err(e) => {
            error!("Failed to get EchoKit servers: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 添加新的 EchoKit Server
pub async fn add_server(
    State(app_state): State<AppState>,
    Json(payload): Json<AddServerRequest>,
) -> Result<Json<ApiResponse<EchoKitServer>>, StatusCode> {
    // TODO: 从认证中间件获取真实的 user_id
    let user_id = "user001"; // 临时使用固定值

    // 验证 URL 格式
    if payload.server_url.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // 插入新服务器
    match sqlx::query_as!(
        EchoKitServer,
        r#"
        INSERT INTO echokit_servers (user_id, server_url)
        VALUES ($1, $2)
        RETURNING
            id,
            user_id as "user_id!",
            server_url as "server_url!",
            created_at as "created_at!",
            updated_at as "updated_at!"
        "#,
        user_id,
        payload.server_url
    )
    .fetch_one(app_state.database.pool())
    .await
    {
        Ok(server) => {
            info!("Added new EchoKit server {} for user {}", server.server_url, user_id);
            Ok(Json(ApiResponse::success(server)))
        }
        Err(e) => {
            // 检查是否是唯一约束冲突
            if let Some(db_err) = e.as_database_error() {
                if db_err.constraint() == Some("unique_user_server_url") {
                    error!("Server URL already exists for user {}: {}", user_id, payload.server_url);
                    return Err(StatusCode::CONFLICT);
                }
            }
            error!("Failed to add EchoKit server: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 删除 EchoKit Server
pub async fn delete_server(
    Path(server_id): Path<i32>,
    State(app_state): State<AppState>,
) -> Result<Json<ApiResponse<serde_json::Value>>, StatusCode> {
    // TODO: 从认证中间件获取真实的 user_id
    let user_id = "user001"; // 临时使用固定值

    // 删除服务器（同时检查所有者）
    match sqlx::query!(
        r#"
        DELETE FROM echokit_servers
        WHERE id = $1 AND user_id = $2
        "#,
        server_id,
        user_id
    )
    .execute(app_state.database.pool())
    .await
    {
        Ok(result) => {
            if result.rows_affected() > 0 {
                info!("Deleted EchoKit server {} for user {}", server_id, user_id);
                Ok(Json(ApiResponse::success(serde_json::json!({
                    "message": "Server deleted successfully",
                    "server_id": server_id
                }))))
            } else {
                error!("Server {} not found or not owned by user {}", server_id, user_id);
                Err(StatusCode::NOT_FOUND)
            }
        }
        Err(e) => {
            error!("Failed to delete EchoKit server: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}


/// EchoKit Server 路由
pub fn echokit_server_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(get_servers).post(add_server))
        .route("/:id", delete(delete_server))
}
