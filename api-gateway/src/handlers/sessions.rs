use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post, delete},
    Router,
};
use echo_shared::{
    ApiResponse, Session, PaginationParams, PaginatedResponse,
    generate_session_id, now_utc, EchoKitConfig, EchoKitSession, EchoKitSessionStatus
};
use echo_shared::types::SessionStatus;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, warn, error};
use crate::app_state::AppState;
use chrono::{DateTime, Utc};
use sqlx::Row;

#[derive(Debug, Deserialize)]
pub struct SessionQueryParams {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub device_id: Option<String>,
    pub status: Option<SessionStatus>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateSessionRequest {
    pub device_id: String,
    pub user_id: String,
    pub config: Option<EchoKitConfig>,
}

#[derive(Debug, Deserialize)]
pub struct EndSessionRequest {
    pub reason: Option<String>,
}

// 用于存储活跃的 EchoKit 会话（实时会话管理）
static mut ECHOKIT_SESSIONS: Option<HashMap<String, EchoKitSession>> = None;

fn get_echokit_sessions() -> &'static mut HashMap<String, EchoKitSession> {
    unsafe {
        if ECHOKIT_SESSIONS.is_none() {
            ECHOKIT_SESSIONS = Some(HashMap::new());
        }
        ECHOKIT_SESSIONS.as_mut().unwrap()
    }
}

// 创建新的 EchoKit 会话
fn create_echokit_session(
    device_id: String,
    user_id: String,
    config: EchoKitConfig,
) -> EchoKitSession {
    EchoKitSession {
        id: generate_session_id(),
        device_id,
        user_id,
        config,
        status: EchoKitSessionStatus::Initializing,
        start_time: now_utc(),
        end_time: None,
        current_stage: echo_shared::SessionStage::Wakeup,
        progress: 0.0,
        transcription: None,
        response: None,
        audio_buffer: Vec::new(),
    }
}

// 模拟调用 Bridge 服务
async fn call_bridge_service_start_session(
    device_id: String,
    user_id: String,
    config: EchoKitConfig,
) -> Result<String, Box<dyn std::error::Error>> {
    // TODO: 实现实际的 Bridge 服务调用
    info!("Simulating Bridge service call - start session for device: {}", device_id);

    // 模拟网络延迟
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // 返回模拟的会话 ID
    Ok(generate_session_id())
}

async fn call_bridge_service_end_session(
    session_id: String,
    reason: String,
) -> Result<(), Box<dyn std::error::Error>> {
    // TODO: 实现实际的 Bridge 服务调用
    info!("Simulating Bridge service call - end session: {} (reason: {})", session_id, reason);

    // 模拟网络延迟
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    Ok(())
}

// ========================================================================
// 历史会话查询（从数据库读取）
// ========================================================================

/// 获取会话列表（支持过滤、分页）
pub async fn get_sessions(
    State(app_state): State<AppState>,
    Query(params): Query<SessionQueryParams>,
) -> Json<ApiResponse<PaginatedResponse<Session>>> {
    let pagination = PaginationParams {
        page: params.page.unwrap_or(1),
        page_size: params.page_size.unwrap_or(20),
    };

    // 构建 SQL 查询条件（使用 SQL 转义避免注入）
    let mut conditions = Vec::new();

    if let Some(device_id) = &params.device_id {
        // 使用 PostgreSQL 的 quote_literal 风格转义
        let escaped = device_id.replace("'", "''");
        conditions.push(format!("device_id = '{}'", escaped));
    }

    if let Some(status) = &params.status {
        let status_str = match status {
            SessionStatus::Active => "active",
            SessionStatus::Completed => "completed",
            SessionStatus::Failed => "failed",
            SessionStatus::Timeout => "timeout",
        };
        conditions.push(format!("status = '{}'", status_str));
    }

    if let Some(start_date) = &params.start_date {
        // 解析 ISO 8601 格式的日期
        if let Ok(start_time) = start_date.parse::<DateTime<Utc>>() {
            conditions.push(format!("start_time >= '{}'", start_time.to_rfc3339()));
        }
    }

    if let Some(end_date) = &params.end_date {
        if let Ok(end_time) = end_date.parse::<DateTime<Utc>>() {
            conditions.push(format!("start_time <= '{}'", end_time.to_rfc3339()));
        }
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    // 查询总数
    let count_query = format!("SELECT COUNT(*) as count FROM sessions {}", where_clause);

    let total: i64 = match sqlx::query(&count_query)
        .fetch_one(app_state.database.pool())
        .await
    {
        Ok(row) => row.get("count"),
        Err(e) => {
            error!("Failed to count sessions: {}", e);
            return Json(ApiResponse::error(format!("Database query failed: {}", e)));
        }
    };

    // 查询分页数据
    let offset = echo_shared::calculate_offset(pagination.page, pagination.page_size);
    let limit = pagination.page_size;

    let data_query = format!(
        "SELECT id, device_id, user_id, start_time, end_time, duration, transcription, response, status
         FROM sessions
         {}
         ORDER BY start_time DESC
         LIMIT {} OFFSET {}",
        where_clause, limit, offset
    );

    let sessions: Vec<Session> = match sqlx::query(&data_query)
        .fetch_all(app_state.database.pool())
        .await
    {
        Ok(rows) => {
            rows.into_iter().map(|row| Session {
                id: row.get("id"),
                device_id: row.get("device_id"),
                user_id: row.get("user_id"),
                start_time: row.get("start_time"),
                end_time: row.get("end_time"),
                duration: row.get("duration"),
                transcription: row.get("transcription"),
                response: row.get("response"),
                status: match row.get::<&str, _>("status") {
                    "active" => SessionStatus::Active,
                    "completed" => SessionStatus::Completed,
                    "failed" => SessionStatus::Failed,
                    "timeout" => SessionStatus::Timeout,
                    _ => SessionStatus::Failed,
                },
            }).collect()
        }
        Err(e) => {
            error!("Failed to query sessions: {}", e);
            return Json(ApiResponse::error(format!("Database query failed: {}", e)));
        }
    };

    let response = PaginatedResponse::new(sessions, total as u64, pagination);
    Json(ApiResponse::success(response))
}

/// 获取单个会话详情
pub async fn get_session(
    Path(session_id): Path<String>,
    State(app_state): State<AppState>,
) -> Result<Json<ApiResponse<Session>>, StatusCode> {
    let query = "SELECT id, device_id, user_id, start_time, end_time, duration, transcription, response, status
                 FROM sessions
                 WHERE id = $1";

    match sqlx::query(query)
        .bind(&session_id)
        .fetch_one(app_state.database.pool())
        .await
    {
        Ok(row) => {
            let session = Session {
                id: row.get("id"),
                device_id: row.get("device_id"),
                user_id: row.get("user_id"),
                start_time: row.get("start_time"),
                end_time: row.get("end_time"),
                duration: row.get("duration"),
                transcription: row.get("transcription"),
                response: row.get("response"),
                status: match row.get::<&str, _>("status") {
                    "active" => SessionStatus::Active,
                    "completed" => SessionStatus::Completed,
                    "failed" => SessionStatus::Failed,
                    "timeout" => SessionStatus::Timeout,
                    _ => SessionStatus::Failed,
                },
            };
            Ok(Json(ApiResponse::success(session)))
        }
        Err(e) => {
            error!("Failed to find session {}: {}", session_id, e);
            Err(StatusCode::NOT_FOUND)
        }
    }
}

/// 获取会话统计信息（从数据库聚合查询）
pub async fn get_session_stats(
    State(app_state): State<AppState>,
) -> Json<ApiResponse<serde_json::Value>> {
    let query = r#"
        SELECT
            COUNT(*) as total,
            COUNT(*) FILTER (WHERE status = 'active') as active,
            COUNT(*) FILTER (WHERE status = 'completed') as completed,
            COUNT(*) FILTER (WHERE status = 'failed') as failed,
            COUNT(*) FILTER (WHERE status = 'timeout') as timeout,
            CAST(AVG(duration) FILTER (WHERE status = 'completed') AS DOUBLE PRECISION) as avg_duration,
            COUNT(*) FILTER (WHERE DATE(start_time) = CURRENT_DATE) as today_sessions
        FROM sessions
    "#;

    match sqlx::query(query)
        .fetch_one(app_state.database.pool())
        .await
    {
        Ok(row) => {
            let avg_duration_f64: Option<f64> = row.get("avg_duration");
            let avg_duration = avg_duration_f64.map(|d| d.round() as i32).unwrap_or(0);

            let stats = json!({
                "total": row.get::<i64, _>("total"),
                "active": row.get::<i64, _>("active"),
                "completed": row.get::<i64, _>("completed"),
                "failed": row.get::<i64, _>("failed"),
                "timeout": row.get::<i64, _>("timeout"),
                "average_duration_seconds": avg_duration,
                "today_sessions": row.get::<i64, _>("today_sessions")
            });

            Json(ApiResponse::success(stats))
        }
        Err(e) => {
            error!("Failed to get session stats: {}", e);
            Json(ApiResponse::error(format!("Database query failed: {}", e)))
        }
    }
}

// ========================================================================
// 实时会话管理（创建、结束会话）
// ========================================================================

/// 创建新会话
pub async fn create_session(
    State(_app_state): State<AppState>,
    Json(payload): Json<CreateSessionRequest>,
) -> Result<Json<ApiResponse<EchoKitSession>>, (StatusCode, Json<ApiResponse<()>>)> {
    let config = payload.config.unwrap_or_default();

    // 检查设备是否已有活跃会话
    {
        let echokit_sessions = get_echokit_sessions();
        for session in echokit_sessions.values() {
            if session.device_id == payload.device_id &&
               (session.status == EchoKitSessionStatus::Active ||
                session.status == EchoKitSessionStatus::Processing ||
                session.status == EchoKitSessionStatus::Responding) {
                let response = ApiResponse::error("Device already has an active session".to_string());
                return Err((StatusCode::CONFLICT, Json(response)));
            }
        }
    }

    // 创建 EchoKit 会话
    let mut echokit_session = create_echokit_session(
        payload.device_id.clone(),
        payload.user_id.clone(),
        config.clone(),
    );

    // 调用 Bridge 服务启动会话
    match call_bridge_service_start_session(
        payload.device_id.clone(),
        payload.user_id.clone(),
        config,
    ).await {
        Ok(_) => {
            // Bridge 服务调用成功，更新会话状态
            echokit_session.status = EchoKitSessionStatus::Active;

            // 存储会话
            let echokit_sessions = get_echokit_sessions();
            echokit_sessions.insert(echokit_session.id.clone(), echokit_session.clone());

            info!("Created new EchoKit session {} for device {}",
                  echokit_session.id, echokit_session.device_id);

            let response = ApiResponse::success(echokit_session);
            Ok(Json(response))
        }
        Err(e) => {
            error!("Failed to create EchoKit session: {}", e);
            let response = ApiResponse::error(format!("Failed to create session: {}", e));
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(response)))
        }
    }
}

/// 更新会话状态（暂不实现，由 Bridge 直接写数据库）
pub async fn update_session(
    Path(_session_id): Path<String>,
    State(_app_state): State<AppState>,
    Json(_payload): Json<serde_json::Value>,
) -> Result<Json<ApiResponse<Session>>, StatusCode> {
    warn!("update_session is deprecated - sessions are now managed directly by Bridge service");
    Err(StatusCode::NOT_IMPLEMENTED)
}

/// 结束会话 (EchoKit 版本)
pub async fn end_session(
    Path(session_id): Path<String>,
    State(_app_state): State<AppState>,
    Json(payload): Json<EndSessionRequest>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    let reason = payload.reason.unwrap_or_else(|| "user_request".to_string());

    // 查找 EchoKit 会话
    let session_info = {
        let echokit_sessions = get_echokit_sessions();
        echokit_sessions.get(&session_id).cloned()
    };

    if let Some(mut session) = session_info {
        // 调用 Bridge 服务结束会话
        match call_bridge_service_end_session(session_id.clone(), reason.clone()).await {
            Ok(_) => {
                // 更新会话状态
                session.status = EchoKitSessionStatus::Completed;
                session.end_time = Some(now_utc());

                // 更新存储
                let echokit_sessions = get_echokit_sessions();
                echokit_sessions.insert(session_id.clone(), session.clone());

                info!("Ended EchoKit session {} (reason: {})", session_id, reason);

                let response = ApiResponse::success(());
                Ok(Json(response))
            }
            Err(e) => {
                error!("Failed to end EchoKit session {}: {}", session_id, e);
                let response = ApiResponse::error(format!("Failed to end session: {}", e));
                Err((StatusCode::INTERNAL_SERVER_ERROR, Json(response)))
            }
        }
    } else {
        let response = ApiResponse::error("Session not found".to_string());
        Err((StatusCode::NOT_FOUND, Json(response)))
    }
}

/// 删除会话（不建议使用，保留数据用于审计）
pub async fn delete_session(
    Path(session_id): Path<String>,
    State(app_state): State<AppState>,
) -> Json<ApiResponse<serde_json::Value>> {
    let query = "DELETE FROM sessions WHERE id = $1";

    match sqlx::query(query)
        .bind(&session_id)
        .execute(app_state.database.pool())
        .await
    {
        Ok(result) => {
            let rows_affected = result.rows_affected();
            if rows_affected > 0 {
                let response = json!({
                    "message": "Session deleted successfully",
                    "session_id": session_id
                });
                Json(ApiResponse::success(response))
            } else {
                Json(ApiResponse::error("Session not found".to_string()))
            }
        }
        Err(e) => {
            error!("Failed to delete session {}: {}", session_id, e);
            Json(ApiResponse::error(format!("Database error: {}", e)))
        }
    }
}

pub fn session_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(get_sessions).post(create_session))
        .route("/stats", get(get_session_stats))
        .route("/:id", get(get_session))
        .route("/:id", post(update_session))
        .route("/:id/end", post(end_session))
        .route("/:id", delete(delete_session))
}
