use echo_shared::AppConfig;
use rumqttc::{AsyncClient, Event, Incoming, MqttOptions, QoS};
use tokio::time::{sleep, Duration};
use tracing::{info, warn, error};
use serde_json;

pub struct MqttClient {
    client: AsyncClient,
}

impl MqttClient {
    pub fn new(config: AppConfig) -> anyhow::Result<Self> {
        let mqtt_host = config.mqtt.broker.clone();
        let mqtt_port = config.mqtt.port;

        let mut mqttoptions = MqttOptions::new("echo-bridge", &mqtt_host, mqtt_port);
        mqttoptions.set_keep_alive(Duration::from_secs(5));

        let (client, eventloop) = AsyncClient::new(mqttoptions, 10);

        // 启动事件循环
        tokio::spawn(run_eventloop(eventloop));

        info!("MQTT client created, connecting to {}:{}", mqtt_host, mqtt_port);

        Ok(MqttClient { client })
    }

    pub async fn publish_device_status(&self, device_id: &str, status: &str) -> anyhow::Result<()> {
        let topic = format!("echo/devices/{}/status", device_id);
        let payload = serde_json::json!({
            "device_id": device_id,
            "status": status,
            "timestamp": chrono::Utc::now().timestamp()
        });

        self.client
            .publish(topic, QoS::AtLeastOnce, false, payload.to_string())
            .await?;

        info!("Published device status for {} to MQTT", device_id);
        Ok(())
    }

    pub async fn publish_audio_data(&self, device_id: &str, audio_data: &[u8]) -> anyhow::Result<()> {
        let topic = format!("echo/devices/{}/audio", device_id);

        self.client
            .publish(topic, QoS::AtLeastOnce, false, audio_data)
            .await?;

        info!("Published audio data for {} to MQTT", device_id);
        Ok(())
    }

    pub async fn subscribe_to_commands(&self) -> anyhow::Result<()> {
        self.client
            .subscribe("echo/commands/+", QoS::AtLeastOnce)
            .await?;

        info!("Subscribed to command topics");
        Ok(())
    }
}

pub async fn start_mqtt_client(config: AppConfig) -> anyhow::Result<()> {
    let mqtt_client = MqttClient::new(config)?;

    // 订阅命令主题
    mqtt_client.subscribe_to_commands().await?;

    // 保持运行
    loop {
        sleep(Duration::from_secs(1)).await;
    }
}

async fn run_eventloop(mut eventloop: rumqttc::EventLoop) {
    info!("Starting MQTT event loop");

    loop {
        match eventloop.poll().await {
            Ok(Event::Incoming(packet)) => {
                handle_incoming_packet(packet).await;
            }
            Ok(_) => {
                info!("Other MQTT event");
            }
            Err(e) => {
                error!("MQTT error: {}", e);
                sleep(Duration::from_secs(5)).await;
            }
        }
    }
}

async fn handle_incoming_packet(packet: Incoming) {
    match packet {
        Incoming::Publish(publish) => {
            info!("Received MQTT message on topic: {}", publish.topic);

            // 处理不同类型的消息
            if publish.topic.contains("commands") {
                handle_command_message(publish).await;
            } else {
                info!("Received message on topic: {} ({} bytes)", publish.topic, publish.payload.len());
            }
        }
        Incoming::ConnAck(connack) => {
            info!("MQTT connection acknowledged: {:?}", connack.code);
        }
        Incoming::PingResp => {
            // 心跳响应
        }
        Incoming::PubAck(puback) => {
            info!("Publish acknowledged: {}", puback.pkid);
        }
        Incoming::PubRec(pubrec) => {
            info!("Publish received: {}", pubrec.pkid);
        }
        Incoming::PubComp(pubcomp) => {
            info!("Publish complete: {}", pubcomp.pkid);
        }
        Incoming::SubAck(suback) => {
            info!("Subscribe acknowledged: {:?}", suback.return_codes);
        }
        Incoming::UnsubAck(unsuback) => {
            info!("Unsubscribe acknowledged: {}", unsuback.pkid);
        }
        _ => {
            info!("Received other MQTT packet: {:?}", packet);
        }
    }
}

async fn handle_command_message(publish: rumqttc::Publish) {
    let payload = String::from_utf8_lossy(&publish.payload);
    info!("Command message received: {}", payload);

    // 解析命令并执行相应的操作
    if let Ok(command) = serde_json::from_str::<serde_json::Value>(&payload) {
        if let Some(cmd_type) = command.get("type").and_then(|v| v.as_str()) {
            match cmd_type {
                "restart" => {
                    info!("Restart command received");
                    // 实现重启逻辑
                }
                "update_config" => {
                    info!("Config update command received");
                    // 实现配置更新逻辑
                }
                _ => {
                    warn!("Unknown command type: {}", cmd_type);
                }
            }
        }
    }
}