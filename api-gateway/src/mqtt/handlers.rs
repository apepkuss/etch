use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use echo_shared::{ApiResponse, DeviceConfiguration, DeviceCommand, MqttConfig};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, error};

use super::client::ApiGatewayMqttClient;

// MQTT 请求/响应类型
#[derive(Debug, Deserialize)]
pub struct DeviceConfigRequest {
    pub device_id: String,
    pub config: DeviceConfiguration,
    pub updated_by: String,
}

#[derive(Debug, Deserialize)]
pub struct DeviceControlRequest {
    pub device_id: String,
    pub command: DeviceCommand,
}

#[derive(Debug, Serialize)]
pub struct MqttStatusResponse {
    pub is_connected: bool,
    pub reconnect_count: u32,
    pub broker_host: String,
    pub broker_port: u16,
}

// MQTT 路由处理器
pub fn mqtt_routes() -> Router<Arc<ApiGatewayMqttClient>> {
    Router::new()
        .route("/status", get(get_mqtt_status))
        .route("/devices/:id/config", post(publish_device_config))
        .route("/devices/:id/control", post(publish_device_control))
}

// 获取 MQTT 状态
pub async fn get_mqtt_status(
    State(mqtt_client): State<Arc<ApiGatewayMqttClient>>,
) -> Json<ApiResponse<MqttStatusResponse>> {
    let is_connected = mqtt_client.is_connected().await;
    let reconnect_count = mqtt_client.get_reconnect_count().await;

    let status = MqttStatusResponse {
        is_connected,
        reconnect_count,
        broker_host: "localhost".to_string(), // TODO: 从配置获取
        broker_port: 1883,
    };

    Json(ApiResponse::success(status))
}

// 发布设备配置
pub async fn publish_device_config(
    State(mqtt_client): State<Arc<ApiGatewayMqttClient>>,
    Path(device_id): Path<String>,
    Json(request): Json<DeviceConfigRequest>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    match mqtt_client
        .publish_device_config(
            request.device_id,
            request.config,
            request.updated_by,
        )
        .await
    {
        Ok(_) => {
            info!("Published device configuration via MQTT");
            let response = ApiResponse::success(());
            Ok(Json(response))
        }
        Err(e) => {
            error!("Failed to publish device configuration: {}", e);
            let response = ApiResponse::error(format!("Failed to publish config: {}", e));
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(response)))
        }
    }
}

// 发布设备控制命令
pub async fn publish_device_control(
    State(mqtt_client): State<Arc<ApiGatewayMqttClient>>,
    Path(device_id): Path<String>,
    Json(request): Json<DeviceControlRequest>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    match mqtt_client
        .publish_device_control(request.device_id, request.command)
        .await
    {
        Ok(_) => {
            info!("Published device control command via MQTT");
            let response = ApiResponse::success(());
            Ok(Json(response))
        }
        Err(e) => {
            error!("Failed to publish device control: {}", e);
            let response = ApiResponse::error(format!("Failed to publish control: {}", e));
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(response)))
        }
    }
}