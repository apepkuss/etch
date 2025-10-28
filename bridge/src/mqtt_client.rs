use anyhow::{Context, Result};
use echo_shared::{
    MqttTopic, MqttPayload, MqttError, TopicFilter,
    DeviceStatus, WakeReason, ServiceStatus, QoS
};
use echo_shared::mqtt::{MqttConfig, MqttMessage};
use echo_shared::utils::now_utc;
use rumqttc::{AsyncClient, Event, Incoming, Outgoing, Packet, QoS as RumqttQoS};
use std::time::Duration as StdDuration;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tracing::{info, warn, error, debug};

// Bridge MQTT 客户端
pub struct BridgeMqttClient {
    client: AsyncClient,
    config: MqttConfig,
    message_sender: mpsc::UnboundedSender<MqttMessage>,
    message_receiver: Arc<RwLock<Option<mpsc::UnboundedReceiver<MqttMessage>>>>,
    registered_devices: Arc<RwLock<std::collections::HashMap<String, DeviceInfo>>>,
    is_connected: Arc<RwLock<bool>>,
    reconnect_count: Arc<RwLock<u32>>,
}

// 设备信息
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub device_id: String,
    pub address: String,
    pub last_seen: chrono::DateTime<chrono::Utc>,
    pub status: DeviceStatus,
    pub user_id: Option<String>,
}

impl BridgeMqttClient {
    pub fn new(config: MqttConfig) -> Result<Self> {
        let mut mqtt_options = rumqttc::MqttOptions::new(
            config.client_id.clone(),
            &config.broker_host,
            config.broker_port,
        );

        // 设置认证信息
        if let (Some(username), Some(password)) = (&config.username, &config.password) {
            mqtt_options.set_credentials(username, password);
        }

        // 设置保持连接
        mqtt_options.set_keep_alive(StdDuration::from_secs(config.keep_alive));
        mqtt_options.set_clean_session(config.clean_session);

        let (client, _) = AsyncClient::new(mqtt_options, 10);

        let (tx, rx) = mpsc::unbounded_channel();

        Ok(Self {
            client,
            config,
            message_sender: tx,
            message_receiver: Arc::new(RwLock::new(Some(rx))),
            registered_devices: Arc::new(RwLock::new(std::collections::HashMap::new())),
            is_connected: Arc::new(RwLock::new(false)),
            reconnect_count: Arc::new(RwLock::new(0)),
        })
    }

    // 启动 MQTT 客户端
    pub async fn start(&self) -> Result<()> {
        info!("Starting MQTT client for Bridge service");

        // 启动消息处理任务
        self.start_message_processor().await?;

        // 启动连接管理任务
        self.start_connection_manager().await?;

        // 启动心跳任务
        self.start_heartbeat_task().await?;

        // 启动设备状态上报任务
        self.start_device_status_reporter().await?;

        Ok(())
    }

    // 发布消息
    pub async fn publish(&self, message: MqttMessage) -> Result<()> {
        let payload = serde_json::to_vec(&message.payload)
            .with_context(|| "Failed to serialize MQTT payload")?;

        let qos = match message.qos {
            QoS::AtMostOnce => RumqttQoS::AtMostOnce,
            QoS::AtLeastOnce => RumqttQoS::AtLeastOnce,
            QoS::ExactlyOnce => RumqttQoS::ExactlyOnce,
        };

        self.client
            .publish(&message.topic, qos, message.retain, payload)
            .await
            .with_context(|| format!("Failed to publish MQTT message to topic: {}", message.topic))?;

        debug!("Published MQTT message to topic: {}", message.topic);
        Ok(())
    }

    // 订阅主题
    pub async fn subscribe(&self, topic_filter: &TopicFilter) -> Result<()> {
        let qos = match topic_filter.qos {
            QoS::AtMostOnce => rumqttc::QoS::AtMostOnce,
            QoS::AtLeastOnce => rumqttc::QoS::AtLeastOnce,
            QoS::ExactlyOnce => rumqttc::QoS::ExactlyOnce,
        };

        self.client
            .subscribe(&topic_filter.topic_pattern, qos)
            .await
            .with_context(|| format!("Failed to subscribe to topic: {}", topic_filter.topic_pattern))?;

        info!("Subscribed to MQTT topic: {}", topic_filter.topic_pattern);
        Ok(())
    }

    // 注册设备
    pub async fn register_device(&self, device_id: String, address: String, user_id: Option<String>) -> Result<()> {
        let device_info = DeviceInfo {
            device_id: device_id.clone(),
            address,
            last_seen: now_utc(),
            status: DeviceStatus::Online,
            user_id,
        };

        // 添加到注册设备列表
        {
            let mut devices = self.registered_devices.write().await;
            devices.insert(device_id.clone(), device_info.clone());
        }

        // 发布设备状态
        self.publish_device_status(&device_id, DeviceStatus::Online, None, None, None).await?;

        info!("Registered device: {}", device_id);
        Ok(())
    }

    // 注销设备
    pub async fn unregister_device(&self, device_id: &str) -> Result<()> {
        // 从注册设备列表中移除
        let removed = {
            let mut devices = self.registered_devices.write().await;
            devices.remove(device_id).is_some()
        };

        if removed {
            // 发布设备离线状态
            self.publish_device_status(device_id, DeviceStatus::Offline, None, None, None).await?;
            info!("Unregistered device: {}", device_id);
        }

        Ok(())
    }

    // 发布设备状态
    pub async fn publish_device_status(
        &self,
        device_id: &str,
        status: DeviceStatus,
        battery_level: Option<i32>,
        volume: Option<i32>,
        location: Option<String>,
    ) -> Result<()> {
        let message = echo_shared::MqttMessageBuilder::device_status(
            device_id.to_string(),
            status,
            battery_level,
            volume,
            location,
        );

        self.publish(message).await
    }

    // 发布设备唤醒事件
    pub async fn publish_device_wake(
        &self,
        device_id: String,
        user_id: Option<String>,
        reason: WakeReason,
    ) -> Result<()> {
        let payload = MqttPayload::DeviceWake {
            device_id: device_id.clone(),
            user_id,
            reason,
            timestamp: now_utc(),
        };

        let message = MqttMessage::new(
            MqttTopic::DeviceWake(device_id).to_string(),
            payload,
            QoS::AtLeastOnce,
        );

        self.publish(message).await
    }

    // 获取已注册的设备列表
    pub async fn get_registered_devices(&self) -> std::collections::HashMap<String, DeviceInfo> {
        self.registered_devices.read().await.clone()
    }

    // 获取设备信息
    pub async fn get_device_info(&self, device_id: &str) -> Option<DeviceInfo> {
        self.registered_devices.read().await.get(device_id).cloned()
    }

    // 检查连接状态
    pub async fn is_connected(&self) -> bool {
        *self.is_connected.read().await
    }

    // 启动消息处理器
    async fn start_message_processor(&self) -> Result<()> {
        let mut receiver = self.message_receiver.write().await.take()
            .ok_or_else(|| anyhow::anyhow!("Message receiver already taken"))?;

        tokio::spawn(async move {
            while let Some(message) = receiver.recv().await {
                if let Err(e) = Self::process_received_message(message).await {
                    error!("Error processing MQTT message: {}", e);
                }
            }
        });

        Ok(())
    }

    // 启动连接管理器
    async fn start_connection_manager(&self) -> Result<()> {
        let client = self.client.clone();
        let config = self.config.clone();
        let message_sender = self.message_sender.clone();
        let is_connected = self.is_connected.clone();
        let reconnect_count = self.reconnect_count.clone();

        tokio::spawn(async move {
            let mut reconnect_attempts = 0;

            loop {
                match Self::run_connection_loop(
                    &client,
                    &message_sender,
                    &is_connected,
                ).await {
                    Ok(_) => {
                        info!("MQTT connection completed normally");
                        break;
                    }
                    Err(e) => {
                        error!("MQTT connection error: {}", e);
                        *is_connected.write().await = false;

                        if reconnect_attempts < config.max_reconnect_attempts {
                            reconnect_attempts += 1;
                            *reconnect_count.write().await = reconnect_attempts;

                            warn!(
                                "Attempting to reconnect to MQTT broker (attempt {}/{}), retrying in {}ms",
                                reconnect_attempts,
                                config.max_reconnect_attempts,
                                config.reconnect_interval_ms
                            );

                            tokio::time::sleep(
                                tokio::time::Duration::from_millis(config.reconnect_interval_ms)
                            ).await;
                        } else {
                            error!("Max MQTT reconnect attempts reached, giving up");
                            break;
                        }
                    }
                }
            }
        });

        Ok(())
    }

    // 启动心跳任务
    async fn start_heartbeat_task(&self) -> Result<()> {
        let client = self.client.clone();
        let service_name = "bridge".to_string();
        let instance_id = format!("{}-{}", service_name, uuid::Uuid::new_v4());

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
            let mut uptime_seconds = 0u64;

            loop {
                interval.tick().await;
                uptime_seconds += 30;

                let heartbeat_message = echo_shared::MqttMessageBuilder::system_heartbeat(
                    service_name.clone(),
                    instance_id.clone(),
                    ServiceStatus::Healthy,
                    uptime_seconds,
                );

                if let Err(e) = Self::publish_heartbeat(&client, heartbeat_message).await {
                    error!("Failed to send MQTT heartbeat: {}", e);
                }
            }
        });

        Ok(())
    }

    // 启动设备状态上报任务
    async fn start_device_status_reporter(&self) -> Result<()> {
        let client = self.client.clone();
        let registered_devices = self.registered_devices.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));

            loop {
                interval.tick().await;

                let devices = registered_devices.read().await.clone();

                for (device_id, device_info) in devices {
                    // 发布设备状态
                    let device_id_clone = device_id.clone();
                    let status_message = echo_shared::MqttMessageBuilder::device_status(
                        device_id,
                        device_info.status,
                        None, // Battery level would be obtained from actual device
                        None, // Volume would be obtained from actual device
                        None, // Location would be obtained from actual device
                    );

                    if let Err(e) = Self::publish_device_status_internal(&client, status_message).await {
                        error!("Failed to publish device status for {}: {}", device_id_clone, e);
                    }
                }
            }
        });

        Ok(())
    }

    // 运行连接循环
    async fn run_connection_loop(
        client: &AsyncClient,
        message_sender: &mpsc::UnboundedSender<MqttMessage>,
        is_connected: &Arc<RwLock<bool>>,
    ) -> Result<()> {
        // TODO: 实现MQTT事件循环 (Phase 3)
        // 暂时简化，避免rumqttc API兼容性问题
        info!("MQTT event loop will be implemented in Phase 3");

        // 保持连接检查的简化版本
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            debug!("MQTT connection check - placeholder for Phase 3 implementation");
        }
    }

    // 解析接收到的消息
    fn parse_incoming_message(received: rumqttc::Publish) -> Result<MqttMessage> {
        let payload: MqttPayload = serde_json::from_slice(&received.payload)
            .with_context(|| "Failed to deserialize MQTT payload")?;

        let qos = match received.qos {
            RumqttQoS::AtMostOnce => QoS::AtMostOnce,
            RumqttQoS::AtLeastOnce => QoS::AtLeastOnce,
            RumqttQoS::ExactlyOnce => QoS::ExactlyOnce,
        };

        Ok(MqttMessage {
            topic: received.topic.clone(),
            payload,
            qos,
            retain: received.retain,
            timestamp: now_utc(),
        })
    }

    // 处理接收到的消息
    async fn process_received_message(message: MqttMessage) -> Result<()> {
        match message.payload {
            MqttPayload::DeviceConfig {
                device_id,
                config,
                updated_by,
                timestamp: _,
            } => {
                info!("Received device configuration for {}: updated by {}", device_id, updated_by);
                // TODO: 应用设备配置
            }
            MqttPayload::DeviceControl {
                device_id,
                command,
                timestamp: _,
            } => {
                info!("Received device control command for {}: {:?}", device_id, command);
                // TODO: 执行设备控制命令
            }
            MqttPayload::SystemStatus {
                service,
                status,
                message,
                details,
                timestamp: _,
            } => {
                info!("System status update: {} - {} ({:?})", service, message, status);
                debug!("System status details: {:?}", details);
            }
            _ => {
                debug!("Received unhandled MQTT message type on topic: {}", message.topic);
            }
        }

        Ok(())
    }

    // 发布心跳消息
    async fn publish_heartbeat(
        client: &AsyncClient,
        message: MqttMessage,
    ) -> Result<()> {
        let payload = serde_json::to_vec(&message.payload)
            .with_context(|| "Failed to serialize heartbeat payload")?;

        let qos = match message.qos {
            QoS::AtMostOnce => RumqttQoS::AtMostOnce,
            QoS::AtLeastOnce => RumqttQoS::AtLeastOnce,
            QoS::ExactlyOnce => RumqttQoS::ExactlyOnce,
        };

        client
            .publish(&message.topic, qos, false, payload)
            .await
            .with_context(|| "Failed to publish heartbeat message")?;

        Ok(())
    }

    // 发布设备状态（内部方法）
    async fn publish_device_status_internal(
        client: &AsyncClient,
        message: MqttMessage,
    ) -> Result<()> {
        let payload = serde_json::to_vec(&message.payload)
            .with_context(|| "Failed to serialize device status payload")?;

        let qos = match message.qos {
            QoS::AtMostOnce => RumqttQoS::AtMostOnce,
            QoS::AtLeastOnce => RumqttQoS::AtLeastOnce,
            QoS::ExactlyOnce => RumqttQoS::ExactlyOnce,
        };

        client
            .publish(&message.topic, qos, message.retain, payload)
            .await
            .with_context(|| "Failed to publish device status message")?;

        Ok(())
    }

    // 获取重连次数
    pub async fn get_reconnect_count(&self) -> u32 {
        *self.reconnect_count.read().await
    }
}