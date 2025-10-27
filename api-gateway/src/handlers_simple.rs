use axum::response::Json;
use echo_shared::{ApiResponse, Device, DeviceType, DeviceStatus, Session, SessionStatus, now_utc};
use serde_json::json;

// 健康检查
pub async fn health_check() -> Json<ApiResponse<serde_json::Value>> {
    let health_data = json!({
        "status": "healthy",
        "timestamp": now_utc().timestamp(),
        "service": "echo-api-gateway",
        "version": "0.1.0"
    });

    Json(ApiResponse::success(health_data))
}

// 获取设备列表
pub async fn get_devices() -> Json<ApiResponse<Vec<Device>>> {
    let devices = vec![
        Device {
            id: "dev001".to_string(),
            name: "客厅音箱".to_string(),
            device_type: DeviceType::Speaker,
            status: DeviceStatus::Online,
            location: "客厅".to_string(),
            firmware_version: "1.2.3".to_string(),
            battery_level: 85,
            volume: 60,
            last_seen: now_utc(),
            is_online: true,
            owner: "user001".to_string(),
        },
        Device {
            id: "dev002".to_string(),
            name: "卧室显示屏".to_string(),
            device_type: DeviceType::Display,
            status: DeviceStatus::Offline,
            location: "主卧室".to_string(),
            firmware_version: "1.2.2".to_string(),
            battery_level: 45,
            volume: 30,
            last_seen: now_utc(),
            is_online: false,
            owner: "user001".to_string(),
        },
        Device {
            id: "dev003".to_string(),
            name: "厨房中控".to_string(),
            device_type: DeviceType::Hub,
            status: DeviceStatus::Online,
            location: "厨房".to_string(),
            firmware_version: "1.2.3".to_string(),
            battery_level: 92,
            volume: 40,
            last_seen: now_utc(),
            is_online: true,
            owner: "user001".to_string(),
        },
    ];

    Json(ApiResponse::success(devices))
}

// 获取设备统计
pub async fn get_device_stats() -> Json<ApiResponse<serde_json::Value>> {
    let stats = json!({
        "total": 3,
        "online": 2,
        "offline": 1,
        "maintenance": 0,
        "error": 0,
        "by_type": {
            "speaker": 1,
            "display": 1,
            "hub": 1
        }
    });

    Json(ApiResponse::success(stats))
}

// 获取会话列表
pub async fn get_sessions() -> Json<ApiResponse<Vec<Session>>> {
    let sessions = vec![
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
    ];

    Json(ApiResponse::success(sessions))
}

// 获取会话统计
pub async fn get_session_stats() -> Json<ApiResponse<serde_json::Value>> {
    let stats = json!({
        "total": 2,
        "active": 1,
        "completed": 1,
        "failed": 0,
        "timeout": 0,
        "average_duration_seconds": 120,
        "today_sessions": 2
    });

    Json(ApiResponse::success(stats))
}

// 登录
pub async fn login() -> Json<ApiResponse<serde_json::Value>> {
    let login_data = json!({
        "token": "mock-jwt-token",
        "user": {
            "id": "admin-001",
            "username": "admin",
            "email": "admin@echo.system",
            "role": "Admin"
        },
        "expires_in": 86400
    });

    Json(ApiResponse::success(login_data))
}