use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post, delete},
    Router,
};
use echo_shared::{ApiResponse, Session, PaginationParams, PaginatedResponse, generate_session_id, now_utc, EchoKitConfig, EchoKitSession, EchoKitSessionStatus};
use echo_shared::types::SessionStatus;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, warn, error, debug};
use crate::app_state::AppState;

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

// 模拟会话数据存储
static mut SESSIONS: Option<Vec<Session>> = None;
static mut ECHOKIT_SESSIONS: Option<HashMap<String, EchoKitSession>> = None;

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

// 获取会话列表
pub async fn get_sessions(
    State(_app_state): State<AppState>,
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
    State(_app_state): State<AppState>,
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

            // 同时创建传统会话记录用于兼容性
            let traditional_session = Session {
                id: echokit_session.id.clone(),
                device_id: echokit_session.device_id.clone(),
                user_id: echokit_session.user_id.clone(),
                start_time: echokit_session.start_time,
                end_time: None,
                duration: None,
                transcription: None,
                response: None,
                status: SessionStatus::Active,
            };

            let sessions = get_mock_sessions();
            sessions.push(traditional_session);

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

// 更新会话状态
pub async fn update_session(
    Path(session_id): Path<String>,
    State(_app_state): State<AppState>,
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

// 结束会话 (EchoKit 版本)
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

                // 同时更新传统会话记录
                let sessions = get_mock_sessions();
                if let Some(traditional_session) = sessions.iter_mut().find(|s| s.id == session_id) {
                    traditional_session.status = SessionStatus::Completed;
                    traditional_session.end_time = Some(now_utc());
                    if let Some(end_time) = traditional_session.end_time {
                        let duration = end_time.signed_duration_since(traditional_session.start_time);
                        traditional_session.duration = Some(duration.num_seconds() as i32);
                    }
                }

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

// 删除会话
pub async fn delete_session(
    Path(session_id): Path<String>,
    State(_app_state): State<AppState>,
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
    State(_app_state): State<AppState>,
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

pub fn session_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(get_sessions).post(create_session))
        .route("/stats", get(get_session_stats))
        .route("/:id", get(get_session))
        .route("/:id", post(update_session))
        .route("/:id/end", post(end_session))
        .route("/:id", delete(delete_session))
}