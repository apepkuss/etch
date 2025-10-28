use anyhow::{Context, Result};
use echo_shared::{
    MqttConfig, MqttMessage, MqttTopic, MqttPayload, MqttError, TopicFilter,
    DeviceStatus, DeviceConfiguration, DeviceCommand, ServiceStatus, now_utc,
    WebSocketMessage, NotificationLevel
};
use rumqttc::{AsyncClient, Event, Incoming, Outgoing, Packet, QoS, SubscribeFilter};
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc, broadcast};
use tracing::{info, warn, error, debug};

// API Gateway MQTT 客户端
pub struct ApiGatewayMqttClient {
    client: AsyncClient,
    config: MqttConfig,
    message_sender: mpsc::UnboundedSender<MqttMessage>,
    message_receiver: Arc<RwLock<Option<mpsc::UnboundedReceiver<MqttMessage>>>>,
    device_status_cache: Arc<RwLock<std::collections::HashMap<String, DeviceStatus>>>,
    websocket_broadcaster: broadcast::Sender<WebSocketMessage>,
    reconnect_count: Arc<RwLock<u32>>,
    is_connected: Arc<RwLock<bool>>,
}

impl ApiGatewayMqttClient {
    pub fn new(config: MqttConfig, websocket_broadcaster: broadcast::Sender<WebSocketMessage>) -> Result<Self> {
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
        mqtt_options.set_keep_alive(config.keep_alive);

        // 设置清理会话
        mqtt_options.set_clean_session(config.clean_session);

        let (client, mut connection) = AsyncClient::new(mqtt_options, 10);

        let (tx, rx) = mpsc::unbounded_channel();

        Ok(Self {
            client,
            config,
            message_sender: tx,
            message_receiver: Arc::new(RwLock::new(Some(rx))),
            device_status_cache: Arc::new(RwLock::new(std::collections::HashMap::new())),
            websocket_broadcaster,
            reconnect_count: Arc::new(RwLock::new(0)),
            is_connected: Arc::new(RwLock::new(false)),
        })
    }

    // 启动 MQTT 客户端
    pub async fn start(&self) -> Result<()> {
        info!("Starting MQTT client for API Gateway");

        // 启动消息处理任务
        self.start_message_processor().await?;

        // 启动连接管理任务
        self.start_connection_manager().await?;

        // 启动心跳任务
        self.start_heartbeat_task().await?;

        Ok(())
    }

    // 发布消息
    pub async fn publish(&self, message: MqttMessage) -> Result<()> {
        let payload = serde_json::to_vec(&message.payload)
            .with_context(|| "Failed to serialize MQTT payload")?;

        let qos = match message.qos {
            QoS::AtMostOnce => rumqttc::QoS::AtMostOnce,
            QoS::AtLeastOnce => rumqttc::QoS::AtLeastOnce,
            QoS::ExactlyOnce => rumqttc::QoS::ExactlyOnce,
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

    // 获取设备状态缓存
    pub async fn get_device_status(&self, device_id: &str) -> Option<DeviceStatus> {
        self.device_status_cache.read().await.get(device_id).cloned()
    }

    // 获取所有设备状态
    pub async fn get_all_device_status(&self) -> std::collections::HashMap<String, DeviceStatus> {
        self.device_status_cache.read().await.clone()
    }

    // 检查连接状态
    pub async fn is_connected(&self) -> bool {
        *self.is_connected.read().await
    }

    // 发布设备配置
    pub async fn publish_device_config(
        &self,
        device_id: String,
        config: DeviceConfiguration,
        updated_by: String,
    ) -> Result<()> {
        let message = echo_shared::MqttMessageBuilder::device_config(
            device_id,
            config,
            updated_by,
        );

        self.publish(message).await
    }

    // 发布设备控制命令
    pub async fn publish_device_control(
        &self,
        device_id: String,
        command: DeviceCommand,
    ) -> Result<()> {
        let message = echo_shared::MqttMessageBuilder::device_control(device_id, command);

        self.publish(message).await
    }

    // 启动消息处理器
    async fn start_message_processor(&self) -> Result<()> {
        let mut receiver = self.message_receiver.write().await.take()
            .ok_or_else(|| anyhow::anyhow!("Message receiver already taken"))?;

        let device_status_cache = self.device_status_cache.clone();
        let websocket_broadcaster = self.websocket_broadcaster.clone();

        tokio::spawn(async move {
            while let Some(message) = receiver.recv().await {
                if let Err(e) = Self::process_received_message(
                    message,
                    &device_status_cache,
                    &websocket_broadcaster,
                ).await {
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
        let service_name = "api-gateway".to_string();
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

    // 运行连接循环
    async fn run_connection_loop(
        client: &AsyncClient,
        message_sender: &mpsc::UnboundedSender<MqttMessage>,
        is_connected: &Arc<RwLock<bool>>,
    ) -> Result<()> {
        let mut notification = client.notifications();

        loop {
            match notification.recv().await {
                Some(event) => match event {
                    Event::Incoming(Packet::Publish(received)) => {
                        debug!("Received MQTT message on topic: {}", received.topic);

                        let mqtt_message = Self::parse_incoming_message(received)?;

                        if let Err(e) = message_sender.send(mqtt_message) {
                            error!("Failed to forward MQTT message: {}", e);
                        }
                    }
                    Event::Incoming(Packet::ConnAck(_)) => {
                        info!("MQTT connection established");
                        *is_connected.write().await = true;
                    }
                    Event::Incoming(Packet::SubAck(suback)) => {
                        info!("MQTT subscription acknowledged: {:?}", suback);
                    }
                    Event::Incoming(Packet::PubAck(_)) => {
                        debug!("MQTT message acknowledged");
                    }
                    Event::Incoming(incoming) => {
                        debug!("Received MQTT packet: {:?}", incoming);
                    }
                    Event::Outgoing(outgoing) => {
                        debug!("Sending MQTT packet: {:?}", outgoing);
                    }
                }
                None => {
                    warn!("MQTT notification stream ended");
                    break;
                }
            }
        }

        Ok(())
    }

    // 解析接收到的消息
    fn parse_incoming_message(received: rumqttc::Publish) -> Result<MqttMessage> {
        let payload: MqttPayload = serde_json::from_slice(&received.payload)
            .with_context(|| "Failed to deserialize MQTT payload")?;

        let qos = match received.qos {
            rumqttc::QoS::AtMostOnce => QoS::AtMostOnce,
            rumqttc::QoS::AtLeastOnce => QoS::AtLeastOnce,
            rumqttc::QoS::ExactlyOnce => QoS::ExactlyOnce,
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
    async fn process_received_message(
        message: MqttMessage,
        device_status_cache: &Arc<RwLock<std::collections::HashMap<String, DeviceStatus>>>,
        websocket_broadcaster: &broadcast::Sender<WebSocketMessage>,
    ) -> Result<()> {
        match message.payload {
            MqttPayload::DeviceStatus {
                device_id,
                status,
                battery_level,
                volume,
                location,
                last_seen,
                metadata: _,
            } => {
                // 更新设备状态缓存
                {
                    let mut cache = device_status_cache.write().await;
                    cache.insert(device_id.clone(), status.clone());
                }

                // 通过 WebSocket 广播设备状态更新
                let ws_message = WebSocketMessage::DeviceStatusUpdate {
                    device_id,
                    status,
                    timestamp: now_utc(),
                };

                if let Err(e) = websocket_broadcaster.send(ws_message) {
                    error!("Failed to broadcast device status via WebSocket: {}", e);
                }

                info!("Updated device status via MQTT");
            }
            MqttPayload::DeviceWake {
                device_id,
                user_id,
                reason,
                timestamp: _,
            } => {
                info!("Device wake event received: {} (reason: {:?})", device_id, reason);

                // 可以在这里触发会话创建逻辑
                // TODO: 集成会话管理
            }
            MqttPayload::SystemStatus {
                service,
                status,
                message,
                details,
                timestamp: _,
            } => {
                info!("System status update: {} - {} ({})", service, message, status);

                // 可以通过 WebSocket 广播系统状态
                if status == ServiceStatus::Unhealthy {
                    let ws_message = WebSocketMessage::SystemNotification {
                        level: NotificationLevel::Error,
                        title: format!("Service Unhealthy: {}", service),
                        message,
                    };

                    if let Err(e) = websocket_broadcaster.send(ws_message) {
                        error!("Failed to broadcast system status via WebSocket: {}", e);
                    }
                }
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
            QoS::AtMostOnce => rumqttc::QoS::AtMostOnce,
            QoS::AtLeastOnce => rumqttc::QoS::AtLeastOnce,
            QoS::ExactlyOnce => rumqttc::QoS::ExactlyOnce,
        };

        client
            .publish(&message.topic, qos, false, payload)
            .await
            .with_context(|| "Failed to publish heartbeat message")?;

        Ok(())
    }

    // 获取重连次数
    pub async fn get_reconnect_count(&self) -> u32 {
        *self.reconnect_count.read().await
    }
}