// 应用程序状态 - 管理所有服务和共享状态
use std::sync::Arc;
use tokio::sync::broadcast::Sender;
use crate::storage::Storage;
use crate::device_service::DeviceService;
use crate::user_service::UserService;
use crate::mqtt::ApiGatewayMqttClient;
use echo_shared::WebSocketMessage;

/// 应用程序状态
#[derive(Clone)]
pub struct AppState {
    pub storage: Arc<Storage>,
    pub device_service: Arc<DeviceService>,
    pub user_service: Arc<UserService>,
    pub mqtt_client: Arc<ApiGatewayMqttClient>,
    pub websocket_tx: Sender<WebSocketMessage>,
}

impl AppState {
    pub fn new(
        storage: Arc<Storage>,
        device_service: Arc<DeviceService>,
        user_service: Arc<UserService>,
        mqtt_client: Arc<ApiGatewayMqttClient>,
        websocket_tx: Sender<WebSocketMessage>,
    ) -> Self {
        Self {
            storage,
            device_service,
            user_service,
            mqtt_client,
            websocket_tx,
        }
    }
}