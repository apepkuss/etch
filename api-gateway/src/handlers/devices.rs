use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post, delete},
    Router,
};
use echo_shared::{ApiResponse, Device, DeviceStatus, DeviceType, DeviceConfig, PaginationParams, PaginatedResponse, generate_uuid, now_utc,
                  DeviceRegistrationRequest, DeviceRegistrationResponse, DeviceVerificationRequest, DeviceVerificationResponse,
                  RegistrationExtensionRequest, RegistrationExtensionResponse};
use tracing::{info, error, warn};
use serde::Deserialize;
use serde_json::json;
use crate::app_state::AppState;

#[derive(Debug, Deserialize)]
pub struct CreateDeviceRequest {
    pub name: String,
    pub device_type: DeviceType,
    pub location: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateDeviceRequest {
    pub name: Option<String>,
    pub location: Option<String>,
    pub config: Option<DeviceConfig>,
}

#[derive(Debug, Deserialize)]
pub struct DeviceQueryParams {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub status: Option<DeviceStatus>,
    pub device_type: Option<DeviceType>,
    pub location: Option<String>,
}

// 模拟设备数据存储
static mut DEVICES: Option<Vec<Device>> = None;

// 获取设备列表
pub async fn get_devices(
    State(app_state): State<AppState>,
    Query(params): Query<DeviceQueryParams>,
) -> Json<ApiResponse<PaginatedResponse<Device>>> {
    let pagination = PaginationParams {
        page: params.page.unwrap_or(1),
        page_size: params.page_size.unwrap_or(20),
    };

    // 从数据库获取设备列表
    match app_state.database.get_all_devices().await {
        Ok(devices) => {
            // 应用过滤条件
            let mut filtered_devices: Vec<Device> = devices;

            if let Some(status) = params.status {
                filtered_devices.retain(|d| d.status == status);
            }

            if let Some(device_type) = params.device_type {
                filtered_devices.retain(|d| d.device_type == device_type);
            }

            if let Some(location) = params.location {
                filtered_devices.retain(|d| d.location.to_lowercase().contains(&location.to_lowercase()));
            }

            // 应用分页
            let total = filtered_devices.len() as u64;
            let offset = echo_shared::calculate_offset(pagination.page, pagination.page_size) as usize;
            let end = (offset + pagination.page_size as usize).min(filtered_devices.len());

            let paginated_devices = if offset < filtered_devices.len() {
                filtered_devices[offset..end].to_vec()
            } else {
                vec![]
            };

            let response = PaginatedResponse::new(paginated_devices, total, pagination);
            Json(ApiResponse::success(response))
        }
        Err(e) => {
            error!("Failed to get devices from database: {}", e);
            let empty_response = PaginatedResponse::new(vec![], 0, pagination);
            Json(ApiResponse::success(empty_response))
        }
    }
}

// 获取单个设备详情
pub async fn get_device(
    Path(device_id): Path<String>,
    State(app_state): State<AppState>,
) -> Result<Json<ApiResponse<Device>>, StatusCode> {
    match app_state.database.get_device_by_id(&device_id).await {
        Ok(Some(device)) => Ok(Json(ApiResponse::success(device))),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            error!("Failed to get device by id {}: {}", device_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// 创建新设备
pub async fn create_device(
    State(app_state): State<AppState>,
    Json(payload): Json<CreateDeviceRequest>,
) -> Json<ApiResponse<Device>> {
    let new_device = Device {
        id: generate_uuid(),
        name: payload.name,
        device_type: payload.device_type,
        status: DeviceStatus::Offline,
        location: payload.location,
        firmware_version: "1.0.0".to_string(),
        battery_level: 100,
        volume: 50,
        last_seen: now_utc(),
        is_online: false,
        owner: "user001".to_string(), // TODO: 从认证信息中获取
    };

    match app_state.database.create_device(
        &new_device,
        None, // serial_number
        None, // mac_address
        None, // pairing_code
        None, // registration_token
    ).await {
        Ok(created_device) => Json(ApiResponse::success(created_device)),
        Err(e) => {
            error!("Failed to create device: {}", e);
            Json(ApiResponse::error("Failed to create device".to_string()))
        }
    }
}

// 更新设备信息
pub async fn update_device(
    Path(device_id): Path<String>,
    State(app_state): State<AppState>,
    Json(payload): Json<UpdateDeviceRequest>,
) -> Result<Json<ApiResponse<Device>>, StatusCode> {
    // 获取现有设备信息
    match app_state.database.get_device_by_id(&device_id).await {
        Ok(Some(mut device)) => {
            // 更新设备信息
            if let Some(name) = payload.name {
                device.name = name;
            }
            if let Some(location) = payload.location {
                device.location = location;
            }
            if let Some(config) = payload.config {
                if let Some(volume) = config.volume {
                    device.volume = volume;
                }
                if let Some(battery_level) = config.battery_level {
                    device.battery_level = battery_level;
                }
            }
            device.last_seen = now_utc();

            // TODO: 实现数据库更新操作
            // let updated_device = app_state.database.update_device(&device).await?;
            // 暂时返回更新后的设备信息
            Ok(Json(ApiResponse::success(device)))
        }
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            error!("Failed to get device for update: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// 删除设备
pub async fn delete_device(
    Path(device_id): Path<String>,
    State(app_state): State<AppState>,
) -> Json<ApiResponse<serde_json::Value>> {
    // 首先检查设备是否存在
    match app_state.database.get_device_by_id(&device_id).await {
        Ok(Some(_device)) => {
            // 实现数据库删除操作
            match app_state.database.delete_device(&device_id).await {
                Ok(()) => {
                    info!("Device {} deleted successfully", device_id);
                    let response = json!({
                        "message": "Device deleted successfully",
                        "device_id": device_id
                    });
                    Json(ApiResponse::success(response))
                }
                Err(e) => {
                    error!("Failed to delete device: {}", e);
                    Json(ApiResponse::error("Failed to delete device".to_string()))
                }
            }
        }
        Ok(None) => {
            Json(ApiResponse::error("Device not found".to_string()))
        }
        Err(e) => {
            error!("Failed to get device for deletion: {}", e);
            Json(ApiResponse::error("Failed to delete device".to_string()))
        }
    }
}

// 重启设备
pub async fn restart_device(
    Path(device_id): Path<String>,
    State(app_state): State<AppState>,
) -> Json<ApiResponse<serde_json::Value>> {
    // 检查设备是否存在
    match app_state.database.get_device_by_id(&device_id).await {
        Ok(Some(_device)) => {
            // TODO: 实现数据库状态更新操作
            // match app_state.database.update_device_status(&device_id, DeviceStatus::Maintenance).await {
            //     Ok(()) => {
            //         // 模拟重启后恢复在线状态
            //         let db_clone = app_state.database.clone();
            //         let device_id_clone = device_id.clone();
            //         tokio::spawn(async move {
            //             tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            //             if let Err(e) = db_clone.update_device_status(&device_id_clone, DeviceStatus::Online).await {
            //                 error!("Failed to restore device status after restart: {}", e);
            //             }
            //         });

            //         let response = json!({
            //             "message": "Device restart initiated",
            //             "device_id": device_id,
            //             "estimated_recovery_time": "5 seconds"
            //         });
            //         Json(ApiResponse::success(response))
            //     }
            //     Err(e) => {
            //         error!("Failed to restart device: {}", e);
            //         Json(ApiResponse::error("Failed to restart device".to_string()))
            //     }
            // }

            // 暂时返回成功响应
            let response = json!({
                "message": "Device restart not yet fully implemented",
                "device_id": device_id,
                "estimated_recovery_time": "5 seconds"
            });
            Json(ApiResponse::success(response))
        }
        Ok(None) => {
            Json(ApiResponse::error("Device not found".to_string()))
        }
        Err(e) => {
            error!("Failed to get device for restart: {}", e);
            Json(ApiResponse::error("Failed to restart device".to_string()))
        }
    }
}

// 获取设备统计信息
pub async fn get_device_stats(
    State(app_state): State<AppState>,
) -> Json<ApiResponse<serde_json::Value>> {
    match app_state.database.get_all_devices().await {
        Ok(devices) => {
            let total = devices.len();
            let online = devices.iter().filter(|d| d.status == DeviceStatus::Online).count();
            let offline = devices.iter().filter(|d| d.status == DeviceStatus::Offline).count();
            let maintenance = devices.iter().filter(|d| d.status == DeviceStatus::Maintenance).count();
            let error = devices.iter().filter(|d| d.status == DeviceStatus::Error).count();
            let pending = devices.iter().filter(|d| d.status == DeviceStatus::Pending).count();

            let stats = json!({
                "total": total,
                "online": online,
                "offline": offline,
                "maintenance": maintenance,
                "error": error,
                "pending": pending,
                "by_type": {
                    "speaker": devices.iter().filter(|d| matches!(d.device_type, DeviceType::Speaker)).count(),
                    "display": 0,
                    "hub": 0
                }
            });

            Json(ApiResponse::success(stats))
        }
        Err(e) => {
            error!("Failed to get devices for stats: {}", e);
            Json(ApiResponse::error("Failed to get device statistics".to_string()))
        }
    }
}

// ================= 设备注册相关API =================

// 注册新设备
pub async fn register_device(
    State(app_state): State<AppState>,
    Json(payload): Json<DeviceRegistrationRequest>,
) -> Result<Json<ApiResponse<DeviceRegistrationResponse>>, StatusCode> {
    // 验证必填字段
    if payload.name.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // 验证序列号和MAC地址是否提供（至少一个）
    if payload.serial_number.is_none() && payload.mac_address.is_none() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // 验证MAC地址格式（如果提供）
    if let Some(ref mac) = payload.mac_address {
        // 支持带冒号或不带冒号的MAC地址格式
        let clean_mac = mac.replace(":", "").replace("-", "");
        if clean_mac.len() != 12 || !clean_mac.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    // 生成配对码和QR令牌
    let pairing_code = generate_pairing_code();
    let qr_token = generate_qr_token();
    let expires_at = chrono::Utc::now() + chrono::Duration::minutes(15);

    // 生成ECHO_<SN>_<MAC>格式的设备ID
    let device_id = match (&payload.serial_number, &payload.mac_address) {
        (Some(sn), Some(mac)) => {
            // 清理MAC地址格式，移除冒号和横线
            let clean_mac = mac.replace(":", "").replace("-", "");
            format!("ECHO_{}_{}", sn, clean_mac)
        }
        (Some(sn), None) => {
            // 如果只有序列号，生成一个占位符MAC
            format!("ECHO_{}_UNKNOWN", sn)
        }
        (None, Some(mac)) => {
            // 如果只有MAC地址，生成一个占位符序列号
            let clean_mac = mac.replace(":", "").replace("-", "");
            format!("ECHO_UNKNOWN_{}", clean_mac)
        }
        (None, None) => {
            // 这种情况已经在前面检查过了
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    // 检查序列号唯一性（如果提供）
    if let Some(ref sn) = payload.serial_number {
        if let Ok(true) = app_state.database.check_serial_number_exists(sn).await {
            return Err(StatusCode::CONFLICT);
        }
    }

    // 检查MAC地址唯一性（如果提供）
    if let Some(ref mac) = payload.mac_address {
        if let Ok(true) = app_state.database.check_mac_address_exists(mac).await {
            return Err(StatusCode::CONFLICT);
        }
    }

    // 创建设备对象
    let new_device = Device {
        id: device_id.clone(),
        name: payload.name.clone(),
        device_type: payload.device_type.clone(),
        status: DeviceStatus::Pending,
        location: "".to_string(), // 默认空字符串，不再从用户输入获取
        firmware_version: "1.0.0".to_string(),
        battery_level: 0,
        volume: 50,
        last_seen: now_utc(),
        is_online: false,
        owner: "user001".to_string(), // TODO: 从认证信息中获取
    };

    // 创建设备和注册令牌
    match app_state.database.create_device(
        &new_device,
        payload.serial_number.as_deref(),
        payload.mac_address.as_deref(),
        Some(&pairing_code),
        Some(&qr_token),
    ).await {
        Ok(_) => {
            // 创建注册令牌记录
            if let Err(e) = app_state.database.create_registration_token(
                &device_id,
                &pairing_code,
                &qr_token,
                expires_at,
            ).await {
                error!("Failed to create registration token: {}", e);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }

            // 生成二维码数据 (使用设备ID进行设备配对)
            let qr_code_data = format!(
                r#"{{"device_id":"{}","pairing_code":"{}","qr_token":"{}","expires_at":"{}","device_type":"{:?}"}}"#,
                device_id, // ECHO_<SN>_<MAC>格式的设备ID
                pairing_code,
                qr_token,
                expires_at.to_rfc3339(),
                payload.device_type
            );

            let registration_response = DeviceRegistrationResponse {
                device_id: device_id.clone(), // 返回ECHO_<SN>_<MAC>格式的设备ID
                pairing_code,
                qr_token,
                qr_code_data,
                expires_at,
                device_type: payload.device_type,
            };

            Ok(Json(ApiResponse::success(registration_response)))
        }
        Err(e) => {
            error!("Failed to create device: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// 验证设备注册
pub async fn verify_device(
    State(app_state): State<AppState>,
    Json(payload): Json<DeviceVerificationRequest>,
) -> Json<ApiResponse<DeviceVerificationResponse>> {
    // 验证配对码
    if payload.pairing_code.is_empty() {
        let verification_response = DeviceVerificationResponse {
            device_id: String::new(),
            success: false,
            message: "配对码不能为空".to_string(),
            device_config: None,
        };
        return Json(ApiResponse::success(verification_response));
    }

    match app_state.database.verify_device_registration(&payload.pairing_code).await {
        Ok(Some(device_id)) => {
            // 获取设备信息
            match app_state.database.get_device_by_id(&device_id).await {
                Ok(Some(device)) => {
                    let verification_response = DeviceVerificationResponse {
                        device_id: device.id.clone(),
                        success: true,
                        message: "设备注册成功".to_string(),
                        device_config: Some(DeviceConfig {
                            volume: Some(50),
                            location: Some(device.location.clone()),
                            battery_level: Some(100),
                        }),
                    };

                    info!("Device registration verified successfully: {}", device_id);
                    Json(ApiResponse::success(verification_response))
                }
                Ok(None) => {
                    let verification_response = DeviceVerificationResponse {
                        device_id: device_id.clone(),
                        success: true,
                        message: "设备注册成功，但无法获取设备信息".to_string(),
                        device_config: Some(DeviceConfig {
                            volume: Some(50),
                            location: None,
                            battery_level: Some(100),
                        }),
                    };
                    Json(ApiResponse::success(verification_response))
                }
                Err(e) => {
                    error!("Failed to get device info after verification: {}", e);
                    let verification_response = DeviceVerificationResponse {
                        device_id: device_id.clone(),
                        success: true,
                        message: "设备注册成功，但获取设备配置失败".to_string(),
                        device_config: None,
                    };
                    Json(ApiResponse::success(verification_response))
                }
            }
        }
        Ok(None) => {
            let verification_response = DeviceVerificationResponse {
                device_id: String::new(),
                success: false,
                message: "配对码无效或已过期".to_string(),
                device_config: None,
            };
            Json(ApiResponse::success(verification_response))
        }
        Err(e) => {
            error!("Failed to verify device registration: {}", e);
            let verification_response = DeviceVerificationResponse {
                device_id: String::new(),
                success: false,
                message: "验证设备注册时发生错误".to_string(),
                device_config: None,
            };
            Json(ApiResponse::success(verification_response))
        }
    }
}

// 延长注册时间
pub async fn extend_registration(
    Path(device_id): Path<String>,
    State(app_state): State<AppState>,
    Json(payload): Json<RegistrationExtensionRequest>,
) -> Json<ApiResponse<RegistrationExtensionResponse>> {
    // 检查设备是否存在且处于待注册状态
    match app_state.database.get_device_by_id(&device_id).await {
        Ok(Some(device)) => {
            if device.status == DeviceStatus::Pending {
                let extension_duration = payload.extension_duration_minutes.unwrap_or(15);
                let new_expires_at = chrono::Utc::now() + chrono::Duration::minutes(extension_duration.into());

                // TODO: 实现数据库中注册令牌延期操作
                // match app_state.database.extend_registration_token(&device_id, new_expires_at).await {
                //     Ok(()) => {
                //         let extension_response = RegistrationExtensionResponse {
                //             success: true,
                //             new_expires_at,
                //             extension_duration_minutes: extension_duration,
                //             message: format!("注册时间已延长{}分钟", extension_duration),
                //         };
                //         Json(ApiResponse::success(extension_response))
                //     }
                //     Err(e) => {
                //         error!("Failed to extend registration: {}", e);
                //         let extension_response = RegistrationExtensionResponse {
                //             success: false,
                //             new_expires_at: chrono::Utc::now(),
                //             extension_duration_minutes: 0,
                //             message: "延长注册时间失败".to_string(),
                //         };
                //         Json(ApiResponse::success(extension_response))
                //     }
                // }

                // 暂时返回成功响应
                let extension_response = RegistrationExtensionResponse {
                    success: true,
                    new_expires_at,
                    extension_duration_minutes: extension_duration,
                    message: format!("注册时间已延长{}分钟 (not fully implemented)", extension_duration),
                };

                Json(ApiResponse::success(extension_response))
            } else {
                let extension_response = RegistrationExtensionResponse {
                    success: false,
                    new_expires_at: chrono::Utc::now(),
                    extension_duration_minutes: 0,
                    message: "设备状态不支持延长".to_string(),
                };

                Json(ApiResponse::success(extension_response))
            }
        }
        Ok(None) => {
            let extension_response = RegistrationExtensionResponse {
                success: false,
                new_expires_at: chrono::Utc::now(),
                extension_duration_minutes: 0,
                message: "设备不存在".to_string(),
            };

            Json(ApiResponse::success(extension_response))
        }
        Err(e) => {
            error!("Failed to get device for registration extension: {}", e);
            Json(ApiResponse::error("Failed to extend registration".to_string()))
        }
    }
}

// 取消注册
pub async fn cancel_registration(
    Path(device_id): Path<String>,
    State(app_state): State<AppState>,
) -> Json<ApiResponse<serde_json::Value>> {
    // 检查设备是否存在且处于待注册状态
    match app_state.database.get_device_by_id(&device_id).await {
        Ok(Some(device)) => {
            if device.status == DeviceStatus::Pending {
                // TODO: 实现数据库状态更新操作
                // match app_state.database.update_device_status(&device_id, DeviceStatus::RegistrationExpired).await {
                //     Ok(()) => {
                //         let response = json!({
                //             "message": "设备注册已取消",
                //             "device_id": device_id,
                //             "device_name": device.name
                //         });
                //         Json(ApiResponse::success(response))
                //     }
                //     Err(e) => {
                //         error!("Failed to cancel registration: {}", e);
                //         Json(ApiResponse::error("Failed to cancel registration".to_string()))
                //     }
                // }

                // 暂时返回成功响应
                let response = json!({
                    "message": "设备注册已取消 (not fully implemented)",
                    "device_id": device_id,
                    "device_name": device.name
                });

                // TODO: 发送WebSocket消息通知前端
                // app_state.websocket_sender.send(WebSocketMessage::DeviceRegistrationExpired { ... }).await?;

                Json(ApiResponse::success(response))
            } else {
                Json(ApiResponse::error("设备状态不支持取消".to_string()))
            }
        }
        Ok(None) => {
            Json(ApiResponse::error("设备不存在".to_string()))
        }
        Err(e) => {
            error!("Failed to get device for registration cancellation: {}", e);
            Json(ApiResponse::error("Failed to cancel registration".to_string()))
        }
    }
}

// 获取待注册设备列表
pub async fn get_pending_registrations(
    State(app_state): State<AppState>,
) -> Json<ApiResponse<Vec<serde_json::Value>>> {
    match app_state.database.get_all_devices().await {
        Ok(devices) => {
            let pending_devices: Vec<serde_json::Value> = devices
                .iter()
                .filter(|d| d.status == DeviceStatus::Pending)
                .map(|d| {
                    json!({
                        "device_id": d.id,
                        "device_name": d.name,
                        "device_type": d.device_type,
                        "location": d.location,
                        "created_at": d.last_seen, // 简化实现
                        "registration_status": "pending"
                    })
                })
                .collect();

            Json(ApiResponse::success(pending_devices))
        }
        Err(e) => {
            error!("Failed to get devices for pending registrations: {}", e);
            Json(ApiResponse::error("Failed to get pending registrations".to_string()))
        }
    }
}

// 生成配对码（简化实现）
fn generate_pairing_code() -> String {
    use rand::Rng;
    let charset = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = rand::thread_rng();

    (0..6)
        .map(|_| {
            let idx = rng.gen_range(0..charset.len());
            charset[idx] as char
        })
        .collect()
}

// 生成QR令牌（简化实现）
fn generate_qr_token() -> String {
    use uuid::Uuid;
    Uuid::new_v4().to_string().replace("-", "")
}

pub fn device_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(get_devices).post(create_device))
        .route("/stats", get(get_device_stats))
        .route("/register", post(register_device))
        .route("/verify", post(verify_device))
        .route("/pending", get(get_pending_registrations))
        .route("/:id/restart", post(restart_device))
        .route("/:id/extend", post(extend_registration))
        .route("/:id/cancel", delete(cancel_registration))
        .route("/:id", get(get_device).put(update_device).delete(delete_device))
}