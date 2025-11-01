// MVP演示版本 - 简化的API Gateway
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::broadcast;

// 简化的应用状态
#[derive(Clone)]
struct AppState {
    devices: Arc<tokio::sync::RwLock<Vec<Value>>>,
    sessions: Arc<tokio::sync::RwLock<Vec<Value>>>,
    ws_tx: broadcast::Sender<String>,
}

impl AppState {
    fn new() -> Self {
        let (ws_tx, _) = broadcast::channel(100);
        Self {
            devices: Arc::new(tokio::sync::RwLock::new(vec![
                json!({
                    "id": "SPEAKER001",
                    "name": "客厅音箱",
                    "type": "智能音箱",
                    "location": "客厅",
                    "status": "online",
                    "battery": 85,
                    "volume": 60,
                    "firmware": "1.2.3"
                }),
                json!({
                    "id": "DISPLAY001",
                    "name": "卧室显示屏",
                    "type": "智能显示屏",
                    "location": "主卧室",
                    "status": "offline",
                    "battery": 45,
                    "volume": 30,
                    "firmware": "1.2.2"
                })
            ])),
            sessions: Arc::new(tokio::sync::RwLock::new(vec![
                json!({
                    "id": "sess001",
                    "device": "客厅音箱",
                    "user": "用户",
                    "time": "2024-10-24 16:25:00",
                    "input": "今天天气怎么样",
                    "output": "今天天气晴朗，温度25度，适合外出活动",
                    "duration": "3.2秒",
                    "status": "completed"
                })
            ])),
            ws_tx,
        }
    }
}

// 健康检查
async fn health_check() -> Result<Json<Value>, StatusCode> {
    Ok(Json(json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "service": "Echo API Gateway MVP",
        "version": "1.0.0"
    })))
}

// 获取设备列表
async fn get_devices(State(state): State<AppState>) -> Json<Value> {
    let devices = state.devices.read().await;
    Json(json!({
        "success": true,
        "data": *devices,
        "total": devices.len()
    }))
}

// 获取会话记录
async fn get_sessions(State(state): State<AppState>) -> Json<Value> {
    let sessions = state.sessions.read().await;
    Json(json!({
        "success": true,
        "data": *sessions,
        "total": sessions.len()
    }))
}

// 获取仪表板数据
async fn get_dashboard(State(state): State<AppState>) -> Json<Value> {
    let devices = state.devices.read().await;
    let sessions = state.sessions.read().await;

    let online_count = devices.iter().filter(|d| d["status"] == "online").count();
    let active_sessions = sessions.iter().filter(|s| s["status"] == "active").count();
    let completed_sessions = sessions.iter().filter(|s| s["status"] == "completed").count();

    Json(json!({
        "success": true,
        "data": {
            "statistics": {
                "total_devices": devices.len(),
                "online_devices": online_count,
                "active_sessions": active_sessions,
                "completed_sessions": completed_sessions,
                "today_sessions": sessions.len()
            },
            "devices": *devices,
            "recent_sessions": *sessions
        }
    }))
}

// 模拟设备控制
async fn control_device(
    State(state): State<AppState>,
    Path(device_id): Path<String>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    let mut devices = state.devices.write().await;

    if let Some(device) = devices.iter_mut().find(|d| d["id"] == device_id) {
        if let Some(action) = payload.get("action").and_then(|v| v.as_str()) {
            match action {
                "restart" => {
                    device["status"] = Value::String("restarting".to_string());
                    // 模拟重启后恢复在线状态
                    let device_id_clone = device_id.clone();
                    let mut devices_clone = devices.clone();
                    tokio::spawn(async move {
                        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                        if let Some(d) = devices_clone.iter_mut().find(|d| d["id"] == device_id_clone) {
                            d["status"] = Value::String("online".to_string());
                        }
                    });

                    Ok(Json(json!({
                        "success": true,
                        "message": format!("设备 {} 正在重启...", device_id)
                    })))
                }
                "configure" => {
                    if let Some(volume) = payload.get("volume").and_then(|v| v.as_u64()) {
                        device["volume"] = Value::Number(serde_json::Number::from(volume));
                    }
                    Ok(Json(json!({
                        "success": true,
                        "message": format!("设备 {} 配置已更新", device_id),
                        "device": *device
                    })))
                }
                _ => Err(StatusCode::BAD_REQUEST)
            }
        } else {
            Err(StatusCode::BAD_REQUEST)
        }
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

// WebSocket测试端点
async fn websocket_test(
    ws: axum::extract::WebSocketUpgrade,
    State(state): State<AppState>,
) -> axum::response::Response {
    ws.on_upgrade(|mut socket| async move {
        let mut rx = state.ws_tx.subscribe();

        // 发送欢迎消息
        let _ = socket.send(axum::extract::ws::Message::Text(
            json!({
                "type": "welcome",
                "message": "欢迎连接到Echo WebSocket服务",
                "timestamp": chrono::Utc::now().to_rfc3339()
            }).to_string()
        )).await;

        // 监听客户端消息和广播消息
        loop {
            tokio::select! {
                // 处理客户端消息
                msg = socket.recv() => {
                    match msg {
                        Some(Ok(axum::extract::ws::Message::Text(text))) => {
                            if let Ok(client_msg) = serde_json::from_str::<Value>(&text) {
                                // 广播消息给其他连接的客户端
                                let broadcast_msg = json!({
                                    "type": "client_message",
                                    "data": client_msg,
                                    "timestamp": chrono::Utc::now().to_rfc3339()
                                }).to_string();
                                let _ = state.ws_tx.send(broadcast_msg);
                            }
                        }
                        Some(Ok(axum::extract::ws::Message::Close(_))) => {
                            break;
                        }
                        _ => {}
                    }
                }
                // 处理广播消息
                broadcast_msg = rx.recv() => {
                    match broadcast_msg {
                        Ok(msg) => {
                            let _ = socket.send(axum::extract::ws::Message::Text(msg)).await;
                        }
                        Err(_) => break,
                    }
                }
            }
        }
    })
}

// 认证相关数据结构
#[derive(Debug, Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Debug, Serialize)]
struct LoginResponse {
    token: String,
    user: UserInfo,
    expires_in: u64,
}

#[derive(Debug, Serialize)]
struct UserInfo {
    id: String,
    username: String,
    email: String,
    role: String,
}

// 简化的登录处理 - 硬编码管理员账户
async fn login(
    Json(payload): Json<LoginRequest>,
) -> Result<Json<Value>, StatusCode> {
    // 简化的硬编码验证
    if payload.username == "admin" && payload.password == "admin123" {
        let token = format!("mock-jwt-token-{}", chrono::Utc::now().timestamp());
        let user_info = UserInfo {
            id: "admin-001".to_string(),
            username: "admin".to_string(),
            email: "admin@echo.system".to_string(),
            role: "admin".to_string(),
        };

        let response = json!({
            "success": true,
            "data": LoginResponse {
                token,
                user: user_info,
                expires_in: 86400, // 24小时
            }
        });

        Ok(Json(response))
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

// 用户信息获取
async fn get_user_info() -> Result<Json<Value>, StatusCode> {
    let user_info = UserInfo {
        id: "admin-001".to_string(),
        username: "admin".to_string(),
        email: "admin@echo.system".to_string(),
        role: "admin".to_string(),
    };

    let response = json!({
        "success": true,
        "data": user_info
    });

    Ok(Json(response))
}

// 退出登录
async fn logout() -> Json<Value> {
    let response = json!({
        "success": true,
        "data": {
            "message": "Logged out successfully"
        }
    });

    Json(response)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 启动Echo API Gateway MVP演示版本...");

    let state = AppState::new();

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/api/auth/login", post(login))
        .route("/api/auth/me", get(get_user_info))
        .route("/api/auth/logout", post(logout))
        .route("/api/devices", get(get_devices))
        .route("/api/sessions", get(get_sessions))
        .route("/api/dashboard", get(get_dashboard))
        .route("/api/devices/:id/control", post(control_device))
        .route("/ws/test", get(websocket_test))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    println!("✅ API Gateway MVP运行在 http://localhost:8080");
    println!("🔗 WebSocket测试端点: ws://localhost:8080/ws/test");
    println!("📊 API端点:");
    println!("   GET  /health - 健康检查");
    println!("   POST /api/auth/login - 用户登录");
    println!("   GET  /api/auth/me - 用户信息");
    println!("   POST /api/auth/logout - 用户退出");
    println!("   GET  /api/devices - 设备列表");
    println!("   GET  /api/sessions - 会话记录");
    println!("   GET  /api/dashboard - 仪表板数据");
    println!("   POST /api/devices/:id/control - 设备控制");

    axum::serve(listener, app).await?;
    Ok(())
}