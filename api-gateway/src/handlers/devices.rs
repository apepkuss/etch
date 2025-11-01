use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post, put, delete},
    Router,
};
use echo_shared::{AppConfig, ApiResponse, Device, DeviceStatus, DeviceType, DeviceConfig, PaginationParams, PaginatedResponse, generate_uuid, now_utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;

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

fn get_mock_devices() -> &'static mut Vec<Device> {
    unsafe {
        if DEVICES.is_none() {
            DEVICES = Some(vec![
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
            ]);
        }
        DEVICES.as_mut().unwrap()
    }
}

// 获取设备列表
pub async fn get_devices(
    State(_config): State<AppConfig>,
    Query(params): Query<DeviceQueryParams>,
) -> Json<ApiResponse<PaginatedResponse<Device>>> {
    let pagination = PaginationParams {
        page: params.page.unwrap_or(1),
        page_size: params.page_size.unwrap_or(20),
    };

    let devices = get_mock_devices();

    // 应用过滤条件
    let mut filtered_devices: Vec<Device> = devices.clone();

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

// 获取单个设备详情
pub async fn get_device(
    Path(device_id): Path<String>,
    State(_config): State<AppConfig>,
) -> Result<Json<ApiResponse<Device>>, StatusCode> {
    let devices = get_mock_devices();

    if let Some(device) = devices.iter().find(|d| d.id == device_id) {
        Ok(Json(ApiResponse::success(device.clone())))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

// 创建新设备
pub async fn create_device(
    State(_config): State<AppConfig>,
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

    let devices = get_mock_devices();
    devices.push(new_device.clone());

    Json(ApiResponse::success(new_device))
}

// 更新设备信息
pub async fn update_device(
    Path(device_id): Path<String>,
    State(_config): State<AppConfig>,
    Json(payload): Json<UpdateDeviceRequest>,
) -> Result<Json<ApiResponse<Device>>, StatusCode> {
    let devices = get_mock_devices();

    if let Some(device) = devices.iter_mut().find(|d| d.id == device_id) {
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

        Ok(Json(ApiResponse::success(device.clone())))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

// 删除设备
pub async fn delete_device(
    Path(device_id): Path<String>,
    State(_config): State<AppConfig>,
) -> Json<ApiResponse<serde_json::Value>> {
    let devices = get_mock_devices();
    let original_len = devices.len();

    devices.retain(|d| d.id != device_id);

    if devices.len() < original_len {
        let response = json!({
            "message": "Device deleted successfully",
            "device_id": device_id
        });
        Json(ApiResponse::success(response))
    } else {
        Json(ApiResponse::error("Device not found".to_string()))
    }
}

// 重启设备
pub async fn restart_device(
    Path(device_id): Path<String>,
    State(_config): State<AppConfig>,
) -> Json<ApiResponse<serde_json::Value>> {
    let devices = get_mock_devices();

    if let Some(device) = devices.iter_mut().find(|d| d.id == device_id) {
        device.status = DeviceStatus::Maintenance;
        device.last_seen = now_utc();

        // 模拟重启后恢复在线状态
        let device_id_clone = device_id.clone();
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            let devices = get_mock_devices();
            if let Some(device) = devices.iter_mut().find(|d| d.id == device_id_clone) {
                device.status = DeviceStatus::Online;
                device.is_online = true;
                device.last_seen = now_utc();
            }
        });

        let response = json!({
            "message": "Device restart initiated",
            "device_id": device_id,
            "estimated_recovery_time": "5 seconds"
        });
        Json(ApiResponse::success(response))
    } else {
        Json(ApiResponse::error("Device not found".to_string()))
    }
}

// 获取设备统计信息
pub async fn get_device_stats(
    State(_config): State<AppConfig>,
) -> Json<ApiResponse<serde_json::Value>> {
    let devices = get_mock_devices();

    let total = devices.len();
    let online = devices.iter().filter(|d| d.status == DeviceStatus::Online).count();
    let offline = devices.iter().filter(|d| d.status == DeviceStatus::Offline).count();
    let maintenance = devices.iter().filter(|d| d.status == DeviceStatus::Maintenance).count();
    let error = devices.iter().filter(|d| d.status == DeviceStatus::Error).count();

    let stats = json!({
        "total": total,
        "online": online,
        "offline": offline,
        "maintenance": maintenance,
        "error": error,
        "by_type": {
            "speaker": devices.iter().filter(|d| matches!(d.device_type, DeviceType::Speaker)).count(),
            "display": devices.iter().filter(|d| matches!(d.device_type, DeviceType::Display)).count(),
            "hub": devices.iter().filter(|d| matches!(d.device_type, DeviceType::Hub)).count()
        }
    });

    Json(ApiResponse::success(stats))
}

pub fn device_routes() -> Router<AppConfig> {
    Router::new()
        .route("/", get(get_devices).post(create_device))
        .route("/stats", get(get_device_stats))
        .route("/:id", get(get_device).put(update_device).delete(delete_device))
        .route("/:id/restart", post(restart_device))
}