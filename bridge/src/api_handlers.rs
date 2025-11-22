// HTTP API handlers for session management

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use echo_shared::{ApiResponse, Session};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, error};
use crate::session::SessionManager;

// API State
#[derive(Clone)]
pub struct ApiState {
    pub session_manager: Arc<SessionManager>,
}

// Request/Response types
#[derive(Debug, Deserialize)]
pub struct CreateSessionRequest {
    pub device_id: String,
    pub user_id: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTranscriptionRequest {
    pub transcription: String,
}

#[derive(Debug, Deserialize)]
pub struct CompleteSessionRequest {
    pub transcription: String,
    pub response: String,
}

// ========================================================================
// API Handlers
// ========================================================================

/// POST /api/sessions - Create a new session
pub async fn create_session(
    State(state): State<ApiState>,
    Json(payload): Json<CreateSessionRequest>,
) -> Result<Json<ApiResponse<Session>>, (StatusCode, Json<ApiResponse<()>>)> {
    info!("API: Creating session for device: {}, user: {}",
          payload.device_id, payload.user_id);

    match state.session_manager.create_session(&payload.device_id, &payload.user_id).await {
        Ok(session) => {
            info!("API: Session created successfully: {}", session.id);
            Ok(Json(ApiResponse::success(session)))
        }
        Err(e) => {
            error!("API: Failed to create session: {}", e);
            let response = ApiResponse::error(format!("Failed to create session: {}", e));
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(response)))
        }
    }
}

/// POST /api/sessions/{id}/transcription - Update session transcription
pub async fn update_transcription(
    Path(session_id): Path<String>,
    State(state): State<ApiState>,
    Json(payload): Json<UpdateTranscriptionRequest>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    info!("API: Updating transcription for session: {}", session_id);

    // Check if session exists
    match state.session_manager.get_session(&session_id).await {
        Some(_) => {
            // Update transcription
            match state.session_manager.update_transcription(&session_id, payload.transcription).await {
                Ok(_) => {
                    info!("API: Transcription updated successfully for session: {}", session_id);
                    Ok(Json(ApiResponse::success(())))
                }
                Err(e) => {
                    error!("API: Failed to update transcription: {}", e);
                    let response = ApiResponse::error(format!("Failed to update transcription: {}", e));
                    Err((StatusCode::INTERNAL_SERVER_ERROR, Json(response)))
                }
            }
        }
        None => {
            error!("API: Session not found: {}", session_id);
            let response = ApiResponse::error("Session not found".to_string());
            Err((StatusCode::NOT_FOUND, Json(response)))
        }
    }
}

/// POST /api/sessions/{id}/complete - Complete a session
pub async fn complete_session(
    Path(session_id): Path<String>,
    State(state): State<ApiState>,
    Json(payload): Json<CompleteSessionRequest>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    info!("API: Completing session: {}", session_id);

    // Check if session exists
    match state.session_manager.get_session(&session_id).await {
        Some(_) => {
            // Complete session
            match state.session_manager.complete_session(
                &session_id,
                payload.transcription,
                payload.response
            ).await {
                Ok(_) => {
                    info!("API: Session completed successfully: {}", session_id);
                    Ok(Json(ApiResponse::success(())))
                }
                Err(e) => {
                    error!("API: Failed to complete session: {}", e);
                    let response = ApiResponse::error(format!("Failed to complete session: {}", e));
                    Err((StatusCode::INTERNAL_SERVER_ERROR, Json(response)))
                }
            }
        }
        None => {
            error!("API: Session not found: {}", session_id);
            let response = ApiResponse::error("Session not found".to_string());
            Err((StatusCode::NOT_FOUND, Json(response)))
        }
    }
}

/// GET /api/sessions/{id} - Get session details
pub async fn get_session(
    Path(session_id): Path<String>,
    State(state): State<ApiState>,
) -> Result<Json<ApiResponse<Session>>, (StatusCode, Json<ApiResponse<()>>)> {
    info!("API: Getting session: {}", session_id);

    match state.session_manager.get_session(&session_id).await {
        Some(session) => {
            info!("API: Session found: {}", session_id);
            Ok(Json(ApiResponse::success(session)))
        }
        None => {
            error!("API: Session not found: {}", session_id);
            let response = ApiResponse::error("Session not found".to_string());
            Err((StatusCode::NOT_FOUND, Json(response)))
        }
    }
}
