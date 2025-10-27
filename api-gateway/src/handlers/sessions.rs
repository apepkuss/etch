use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use echo_shared::{AppConfig, ApiResponse, Session, SessionStatus, PaginationParams, PaginatedResponse, generate_uuid, now_utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;

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
}

// 模拟会话数据存储
static mut SESSIONS: Option<Vec<Session>> = None;

fn get_mock_sessions() -> &'static mut Vec<Session> {
    unsafe {
        if SESSIONS.is_none() {
            SESSIONS = Some(vec![
                Session {
                    id: "sess001".to_string(),
                    device_id: "dev001".to_string(),
                    user_id: "user001".to_string(),
                    start_time: now_utc(),
                    end_time: Some(now_utc()),
                    duration: Some(120),
                    transcription: Some("今天天气怎么样".to_string()),
                    response: Some("今天天气晴朗，温度25摄氏度，适合外出活动。".to_string()),
                    status: SessionStatus::Completed,
                },
                Session {
                    id: "sess002".to_string(),
                    device_id: "dev002".to_string(),
                    user_id: "user001".to_string(),
                    start_time: now_utc(),
                    end_time: None,
                    duration: None,
                    transcription: Some("播放一些音乐".to_string()),
                    response: None,
                    status: SessionStatus::Active,
                },
                Session {
                    id: "sess003".to_string(),
                    device_id: "dev003".to_string(),
                    user_id: "user001".to_string(),
                    start_time: now_utc(),
                    end_time: Some(now_utc()),
                    duration: Some(30),
                    transcription: Some("设置闹钟".to_string()),
                    response: Some("好的，已为您设置闹钟。".to_string()),
                    status: SessionStatus::Completed,
                },
            ]);
        }
        SESSIONS.as_mut().unwrap()
    }
}

// 获取会话列表
pub async fn get_sessions(
    State(_config): State<AppConfig>,
    Query(params): Query<SessionQueryParams>,
) -> Json<ApiResponse<PaginatedResponse<Session>>> {
    let pagination = PaginationParams {
        page: params.page.unwrap_or(1),
        page_size: params.page_size.unwrap_or(20),
    };

    let sessions = get_mock_sessions();

    // 应用过滤条件
    let mut filtered_sessions: Vec<Session> = sessions.clone();

    if let Some(device_id) = params.device_id {
        filtered_sessions.retain(|s| s.device_id == device_id);
    }

    if let Some(status) = params.status {
        filtered_sessions.retain(|s| s.status == status);
    }

    // TODO: 实现日期范围过滤

    // 按开始时间倒序排列
    filtered_sessions.sort_by(|a, b| b.start_time.cmp(&a.start_time));

    // 应用分页
    let total = filtered_sessions.len() as u64;
    let offset = echo_shared::calculate_offset(pagination.page, pagination.page_size) as usize;
    let end = (offset + pagination.page_size as usize).min(filtered_sessions.len());

    let paginated_sessions = if offset < filtered_sessions.len() {
        filtered_sessions[offset..end].to_vec()
    } else {
        vec![]
    };

    let response = PaginatedResponse::new(paginated_sessions, total, pagination);
    Json(ApiResponse::success(response))
}

// 获取单个会话详情
pub async fn get_session(
    Path(session_id): Path<String>,
    State(_config): State<AppConfig>,
) -> Result<Json<ApiResponse<Session>>, StatusCode> {
    let sessions = get_mock_sessions();

    if let Some(session) = sessions.iter().find(|s| s.id == session_id) {
        Ok(Json(ApiResponse::success(session.clone())))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

// 创建新会话
pub async fn create_session(
    State(_config): State<AppConfig>,
    Json(payload): Json<CreateSessionRequest>,
) -> Json<ApiResponse<Session>> {
    let new_session = Session {
        id: generate_uuid(),
        device_id: payload.device_id,
        user_id: payload.user_id,
        start_time: now_utc(),
        end_time: None,
        duration: None,
        transcription: None,
        response: None,
        status: SessionStatus::Active,
    };

    let sessions = get_mock_sessions();
    sessions.push(new_session.clone());

    Json(ApiResponse::success(new_session))
}

// 更新会话状态
pub async fn update_session(
    Path(session_id): Path<String>,
    State(_config): State<AppConfig>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<ApiResponse<Session>>, StatusCode> {
    let sessions = get_mock_sessions();

    if let Some(session) = sessions.iter_mut().find(|s| s.id == session_id) {
        // 更新会话信息
        if let Some(transcription) = payload.get("transcription").and_then(|v| v.as_str()) {
            session.transcription = Some(transcription.to_string());
        }
        if let Some(response) = payload.get("response").and_then(|v| v.as_str()) {
            session.response = Some(response.to_string());
        }
        if let Some(status) = payload.get("status").and_then(|v| v.as_str()) {
            session.status = match status {
                "active" => SessionStatus::Active,
                "completed" => SessionStatus::Completed,
                "failed" => SessionStatus::Failed,
                "timeout" => SessionStatus::Timeout,
                _ => session.status.clone(),
            };
        }

        // 如果会话完成，更新结束时间和持续时间
        if session.status == SessionStatus::Completed && session.end_time.is_none() {
            session.end_time = Some(now_utc());
            if let Some(end_time) = session.end_time {
                let duration = end_time.signed_duration_since(session.start_time);
                session.duration = Some(duration.num_seconds() as i32);
            }
        }

        Ok(Json(ApiResponse::success(session.clone())))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

// 删除会话
pub async fn delete_session(
    Path(session_id): Path<String>,
    State(_config): State<AppConfig>,
) -> Json<ApiResponse<serde_json::Value>> {
    let sessions = get_mock_sessions();
    let original_len = sessions.len();

    sessions.retain(|s| s.id != session_id);

    if sessions.len() < original_len {
        let response = json!({
            "message": "Session deleted successfully",
            "session_id": session_id
        });
        Json(ApiResponse::success(response))
    } else {
        Json(ApiResponse::error("Session not found".to_string()))
    }
}

// 获取会话统计信息
pub async fn get_session_stats(
    State(_config): State<AppConfig>,
) -> Json<ApiResponse<serde_json::Value>> {
    let sessions = get_mock_sessions();

    let total = sessions.len();
    let active = sessions.iter().filter(|s| s.status == SessionStatus::Active).count();
    let completed = sessions.iter().filter(|s| s.status == SessionStatus::Completed).count();
    let failed = sessions.iter().filter(|s| s.status == SessionStatus::Failed).count();
    let timeout = sessions.iter().filter(|s| s.status == SessionStatus::Timeout).count();

    // 计算平均持续时间
    let completed_sessions: Vec<_> = sessions.iter()
        .filter(|s| s.status == SessionStatus::Completed && s.duration.is_some())
        .collect();

    let avg_duration = if !completed_sessions.is_empty() {
        let total_duration: i32 = completed_sessions.iter()
            .map(|s| s.duration.unwrap())
            .sum();
        total_duration / completed_sessions.len() as i32
    } else {
        0
    };

    let stats = json!({
        "total": total,
        "active": active,
        "completed": completed,
        "failed": failed,
        "timeout": timeout,
        "average_duration_seconds": avg_duration,
        "today_sessions": sessions.iter().filter(|s| {
            s.start_time.date_naive() == now_utc().date_naive()
        }).count()
    });

    Json(ApiResponse::success(stats))
}

pub fn session_routes() -> Router<AppConfig> {
    Router::new()
        .route("/sessions", get(get_sessions).post(create_session))
        .route("/sessions/stats", get(get_session_stats))
        .route("/sessions/:id", get(get_session).delete(delete_session))
        .route("/sessions/:id", post(update_session))
}