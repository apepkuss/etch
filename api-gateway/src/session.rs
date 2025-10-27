use anyhow::{Context, Result};
use echo_shared::{
    Session, SessionStatus, SessionStage, EchoKitConfig, EchoKitSession, EchoKitSessionStatus,
    WebSocketMessage, now_utc, generate_session_id, ApiResponse, EchoError
};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Json;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error, debug};

// 会话管理器
#[derive(Clone)]
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<String, EchoKitSession>>>,
    bridge_client: Arc<BridgeClient>,
}

// Bridge 客户端
pub struct BridgeClient {
    bridge_url: String,
    http_client: reqwest::Client,
}

impl BridgeClient {
    pub fn new(bridge_url: String) -> Self {
        Self {
            bridge_url,
            http_client: reqwest::Client::new(),
        }
    }

    // 启动会话
    pub async fn start_session(
        &self,
        device_id: String,
        user_id: String,
        config: Option<EchoKitConfig>,
    ) -> Result<String> {
        let url = format!("{}/sessions/start", self.bridge_url);

        let mut body = serde_json::json!({
            "device_id": device_id,
            "user_id": user_id,
        });

        if let Some(config) = config {
            body["config"] = serde_json::to_value(config)?;
        }

        let response = self.http_client
            .post(&url)
            .json(&body)
            .send()
            .await
            .with_context(|| "Failed to send start session request to Bridge")?;

        if response.status().is_success() {
            let result: serde_json::Value = response.json().await
                .with_context(|| "Failed to parse Bridge response")?;

            if let Some(session_id) = result.get("session_id").and_then(|v| v.as_str()) {
                info!("Started session {} via Bridge service", session_id);
                return Ok(session_id.to_string());
            }
        }

        Err(anyhow::anyhow!("Bridge service returned error: {}", response.status()))
    }

    // 结束会话
    pub async fn end_session(&self, session_id: &str, reason: &str) -> Result<()> {
        let url = format!("{}/sessions/{}/end", self.bridge_url, session_id);

        let body = serde_json::json!({
            "reason": reason
        });

        let response = self.http_client
            .post(&url)
            .json(&body)
            .send()
            .await
            .with_context(|| "Failed to send end session request to Bridge")?;

        if response.status().is_success() {
            info!("Ended session {} via Bridge service", session_id);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Bridge service returned error: {}", response.status()))
        }
    }

    // 获取 Bridge 服务状态
    pub async fn get_stats(&self) -> Result<BridgeStats> {
        let url = format!("{}/stats", self.bridge_url);

        let response = self.http_client
            .get(&url)
            .send()
            .await
            .with_context(|| "Failed to get Bridge stats")?;

        if response.status().is_success() {
            let stats: BridgeStats = response.json().await
                .with_context(|| "Failed to parse Bridge stats")?;
            Ok(stats)
        } else {
            Err(anyhow::anyhow!("Failed to get Bridge stats: {}", response.status()))
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BridgeStats {
    pub echokit_connected: bool,
    pub echokit_sessions: usize,
    pub bridge_sessions: usize,
    pub audio_sessions: usize,
    pub online_devices: usize,
    pub uptime_seconds: u64,
}

impl SessionManager {
    pub fn new(bridge_url: String) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            bridge_client: Arc::new(BridgeClient::new(bridge_url)),
        }
    }

    // 创建新会话
    pub async fn create_session(
        &self,
        device_id: String,
        user_id: String,
        config: Option<EchoKitConfig>,
    ) -> Result<EchoKitSession> {
        let session_id = generate_session_id();
        let echokit_config = config.unwrap_or_default();

        // 检查设备是否已有活跃会话
        {
            let sessions = self.sessions.read().await;
            for session in sessions.values() {
                if session.device_id == device_id && session.status == EchoKitSessionStatus::Active {
                    return Err(anyhow::anyhow!("Device already has an active session"));
                }
            }
        }

        // 通过 Bridge 服务启动会话
        self.bridge_client.start_session(
            device_id.clone(),
            user_id.clone(),
            Some(echokit_config.clone()),
        ).await?;

        // 创建本地会话记录
        let session = EchoKitSession {
            id: session_id.clone(),
            device_id: device_id.clone(),
            user_id,
            config: echokit_config,
            status: EchoKitSessionStatus::Initializing,
            start_time: now_utc(),
            end_time: None,
            current_stage: SessionStage::Wakeup,
            progress: 0.0,
            transcription: None,
            response: None,
            audio_buffer: Vec::new(),
        };

        // 添加到会话管理器
        self.sessions.write().await.insert(session_id.clone(), session.clone());

        info!("Created new session {} for device {}", session_id, device_id);
        Ok(session)
    }

    // 结束会话
    pub async fn end_session(&self, session_id: &str, reason: &str) -> Result<()> {
        let session = {
            let sessions = self.sessions.read().await;
            sessions.get(session_id).cloned()
        };

        if let Some(session) = session {
            // 通过 Bridge 服务结束会话
            if let Err(e) = self.bridge_client.end_session(session_id, reason).await {
                error!("Failed to end session via Bridge: {}", e);
            }

            // 更新本地会话状态
            {
                let mut sessions = self.sessions.write().await;
                if let Some(s) = sessions.get_mut(session_id) {
                    s.status = EchoKitSessionStatus::Completed;
                    s.end_time = Some(now_utc());
                }
            }

            info!("Ended session {} (reason: {})", session_id, reason);
        } else {
            warn!("Session {} not found", session_id);
            return Err(anyhow::anyhow!("Session not found"));
        }

        Ok(())
    }

    // 获取会话信息
    pub async fn get_session(&self, session_id: &str) -> Option<EchoKitSession> {
        self.sessions.read().await.get(session_id).cloned()
    }

    // 获取设备当前会话
    pub async fn get_device_session(&self, device_id: &str) -> Option<EchoKitSession> {
        let sessions = self.sessions.read().await;
        sessions.values()
            .find(|s| s.device_id == device_id && s.status == EchoKitSessionStatus::Active)
            .cloned()
    }

    // 获取用户的所有会话
    pub async fn get_user_sessions(&self, user_id: &str) -> Vec<EchoKitSession> {
        self.sessions.read().await
            .values()
            .filter(|s| s.user_id == user_id)
            .cloned()
            .collect()
    }

    // 更新会话状态
    pub async fn update_session_status(
        &self,
        session_id: &str,
        status: EchoKitSessionStatus,
    ) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.status = status;
            info!("Updated session {} status to {:?}", session_id, status);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Session not found"))
        }
    }

    // 更新会话阶段
    pub async fn update_session_stage(
        &self,
        session_id: &str,
        stage: SessionStage,
        progress: f32,
        message: Option<String>,
    ) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.current_stage = stage;
            session.progress = progress;

            if let Some(msg) = message {
                // 可以将消息存储到转录或响应中
                if stage == SessionStage::Listening {
                    session.transcription = Some(msg);
                } else if stage == SessionStage::Responding {
                    session.response = Some(msg);
                }
            }

            debug!("Updated session {} stage to {:?} (progress: {:.1}%)",
                   session_id, stage, progress);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Session not found"))
        }
    }

    // 更新转录结果
    pub async fn update_transcription(
        &self,
        session_id: &str,
        transcription: String,
    ) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.transcription = Some(transcription);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Session not found"))
        }
    }

    // 更新响应结果
    pub async fn update_response(
        &self,
        session_id: &str,
        response: String,
    ) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.response = Some(response);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Session not found"))
        }
    }

    // 清理过期会话
    pub async fn cleanup_expired_sessions(&self, timeout_hours: i64) -> Result<usize> {
        let now = now_utc();
        let mut sessions_to_remove = Vec::new();

        {
            let sessions = self.sessions.read().await;
            for (session_id, session) in sessions.iter() {
                let duration = now.signed_duration_since(session.start_time);
                if duration.num_hours() > timeout_hours {
                    sessions_to_remove.push(session_id.clone());
                }
            }
        }

        // 移除过期会话
        let mut count = 0;
        {
            let mut sessions = self.sessions.write().await;
            for session_id in &sessions_to_remove {
                if sessions.remove(session_id).is_some() {
                    count += 1;
                    debug!("Removed expired session: {}", session_id);
                }
            }
        }

        if count > 0 {
            info!("Cleaned up {} expired sessions", count);
        }

        Ok(count)
    }

    // 获取会话统计信息
    pub async fn get_session_stats(&self) -> SessionStats {
        let sessions = self.sessions.read().await;
        let mut stats = SessionStats::default();

        for session in sessions.values() {
            stats.total_sessions += 1;

            match session.status {
                EchoKitSessionStatus::Active | EchoKitSessionStatus::Processing | EchoKitSessionStatus::Responding => {
                    stats.active_sessions += 1;
                }
                EchoKitSessionStatus::Completed => {
                    stats.completed_sessions += 1;
                }
                EchoKitSessionStatus::Failed => {
                    stats.failed_sessions += 1;
                }
                _ => {}
            }
        }

        stats
    }

    // 获取 Bridge 服务统计
    pub async fn get_bridge_stats(&self) -> Result<BridgeStats> {
        self.bridge_client.get_stats().await
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct SessionStats {
    pub total_sessions: u64,
    pub active_sessions: u64,
    pub completed_sessions: u64,
    pub failed_sessions: u64,
}

// HTTP 请求/响应类型
#[derive(Debug, Deserialize)]
pub struct CreateSessionRequest {
    pub device_id: String,
    pub config: Option<EchoKitConfig>,
}

#[derive(Debug, Deserialize)]
pub struct EndSessionRequest {
    pub reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SessionResponse {
    pub session: EchoKitSession,
}

// HTTP 处理器
impl SessionManager {
    // 创建会话 HTTP 处理器
    pub async fn create_session_handler(
        State(session_manager): State<Arc<SessionManager>>,
        Json(request): Json<CreateSessionRequest>,
    ) -> Result<Json<ApiResponse<EchoKitSession>>, (StatusCode, Json<ApiResponse<()>>)> {
        // TODO: 从 JWT token 中获取用户 ID
        let user_id = "user001".to_string();

        match session_manager.create_session(
            request.device_id,
            user_id,
            request.config,
        ).await {
            Ok(session) => {
                let response = ApiResponse::success(session);
                Ok(Json(response))
            }
            Err(e) => {
                error!("Failed to create session: {}", e);
                let response = ApiResponse::error(format!("Failed to create session: {}", e));
                Err((StatusCode::INTERNAL_SERVER_ERROR, Json(response)))
            }
        }
    }

    // 结束会话 HTTP 处理器
    pub async fn end_session_handler(
        State(session_manager): State<Arc<SessionManager>>,
        Path(session_id): Path<String>,
        Json(request): Json<EndSessionRequest>,
    ) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
        let reason = request.reason.unwrap_or_else(|| "user_request".to_string());

        match session_manager.end_session(&session_id, &reason).await {
            Ok(_) => {
                let response = ApiResponse::success(());
                Ok(Json(response))
            }
            Err(e) => {
                error!("Failed to end session {}: {}", session_id, e);
                let response = ApiResponse::error(format!("Failed to end session: {}", e));
                Err((StatusCode::INTERNAL_SERVER_ERROR, Json(response)))
            }
        }
    }

    // 获取会话信息 HTTP 处理器
    pub async fn get_session_handler(
        State(session_manager): State<Arc<SessionManager>>,
        Path(session_id): Path<String>,
    ) -> Result<Json<ApiResponse<EchoKitSession>>, (StatusCode, Json<ApiResponse<()>>)> {
        match session_manager.get_session(&session_id).await {
            Some(session) => {
                let response = ApiResponse::success(session);
                Ok(Json(response))
            }
            None => {
                let response = ApiResponse::error("Session not found".to_string());
                Err((StatusCode::NOT_FOUND, Json(response)))
            }
        }
    }

    // 获取用户会话列表 HTTP 处理器
    pub async fn get_user_sessions_handler(
        State(session_manager): State<Arc<SessionManager>>,
    ) -> Result<Json<ApiResponse<Vec<EchoKitSession>>>, (StatusCode, Json<ApiResponse<()>>)> {
        // TODO: 从 JWT token 中获取用户 ID
        let user_id = "user001".to_string();

        let sessions = session_manager.get_user_sessions(&user_id).await;
        let response = ApiResponse::success(sessions);
        Ok(Json(response))
    }

    // 获取会话统计 HTTP 处理器
    pub async fn get_session_stats_handler(
        State(session_manager): State<Arc<SessionManager>>,
    ) -> Json<ApiResponse<SessionStats>> {
        let stats = session_manager.get_session_stats().await;
        let response = ApiResponse::success(stats);
        Json(response)
    }
}