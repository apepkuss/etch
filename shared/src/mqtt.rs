use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use crate::{DeviceStatus};

mod qos_serde {
    use serde::{Deserialize, Deserializer, Serializer};
    use super::QoS;

    pub fn serialize<S>(qos: &QoS, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u8(*qos as u8)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<QoS, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = u8::deserialize(deserializer)?;
        match value {
            0 => Ok(QoS::AtMostOnce),
            1 => Ok(QoS::AtLeastOnce),
            2 => Ok(QoS::ExactlyOnce),
            _ => Err(serde::de::Error::custom(format!("Invalid QoS value: {}", value))),
        }
    }
}

// MQTT 消息质量等级
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QoS {
    AtMostOnce = 0,
    AtLeastOnce = 1,
    ExactlyOnce = 2,
}

// MQTT 主题定义
#[derive(Debug, Clone, PartialEq)]
pub enum MqttTopic {
    // 设备相关主题
    DeviceWake(String),        // device/{device_id}/wake
    DeviceStatus(String),      // device/{device_id}/status
    DeviceConfig(String),      // device/{device_id}/config
    DeviceControl(String),     // device/{device_id}/control

    // 系统相关主题
    SystemHeartbeat(String),   // system/{service}/heartbeat
    SystemStatus(String),      // system/{service}/status

    // 用户相关主题
    UserNotification(String),  // user/{user_id}/notification

    // 通用主题
    Broadcast(String),         // broadcast/{message_type}
}

impl MqttTopic {
    /// 构建主题字符串
    pub fn to_string(&self) -> String {
        match self {
            MqttTopic::DeviceWake(device_id) => format!("device/{}/wake", device_id),
            MqttTopic::DeviceStatus(device_id) => format!("device/{}/status", device_id),
            MqttTopic::DeviceConfig(device_id) => format!("device/{}/config", device_id),
            MqttTopic::DeviceControl(device_id) => format!("device/{}/control", device_id),
            MqttTopic::SystemHeartbeat(service) => format!("system/{}/heartbeat", service),
            MqttTopic::SystemStatus(service) => format!("system/{}/status", service),
            MqttTopic::UserNotification(user_id) => format!("user/{}/notification", user_id),
            MqttTopic::Broadcast(message_type) => format!("broadcast/{}", message_type),
        }
    }

    /// 从主题字符串解析
    pub fn from_string(topic: &str) -> Option<Self> {
        let parts: Vec<&str> = topic.split('/').collect();

        match parts.as_slice() {
            ["device", device_id, "wake"] => Some(MqttTopic::DeviceWake(device_id.to_string())),
            ["device", device_id, "status"] => Some(MqttTopic::DeviceStatus(device_id.to_string())),
            ["device", device_id, "config"] => Some(MqttTopic::DeviceConfig(device_id.to_string())),
            ["device", device_id, "control"] => Some(MqttTopic::DeviceControl(device_id.to_string())),
            ["system", service, "heartbeat"] => Some(MqttTopic::SystemHeartbeat(service.to_string())),
            ["system", service, "status"] => Some(MqttTopic::SystemStatus(service.to_string())),
            ["user", user_id, "notification"] => Some(MqttTopic::UserNotification(user_id.to_string())),
            ["broadcast", message_type] => Some(MqttTopic::Broadcast(message_type.to_string())),
            _ => None,
        }
    }

    /// 获取主题中的设备ID
    pub fn get_device_id(&self) -> Option<String> {
        match self {
            MqttTopic::DeviceWake(device_id) |
            MqttTopic::DeviceStatus(device_id) |
            MqttTopic::DeviceConfig(device_id) |
            MqttTopic::DeviceControl(device_id) => Some(device_id.clone()),
            _ => None,
        }
    }
}

// MQTT 消息结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttMessage {
    pub topic: String,
    pub payload: MqttPayload,
    #[serde(with = "qos_serde")]
    pub qos: QoS,
    pub retain: bool,
    pub timestamp: DateTime<Utc>,
}

impl MqttMessage {
    pub fn new(topic: String, payload: MqttPayload, qos: QoS) -> Self {
        Self {
            topic,
            payload,
            qos,
            retain: false,
            timestamp: Utc::now(),
        }
    }

    pub fn with_retain(mut self, retain: bool) -> Self {
        self.retain = retain;
        self
    }
}

// MQTT 消息负载类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MqttPayload {
    // 设备唤醒消息
    DeviceWake {
        device_id: String,
        user_id: Option<String>,
        reason: WakeReason,
        timestamp: DateTime<Utc>,
    },

    // 设备状态消息
    DeviceStatus {
        device_id: String,
        status: DeviceStatus,
        battery_level: Option<i32>,
        volume: Option<i32>,
        location: Option<String>,
        last_seen: DateTime<Utc>,
        metadata: Option<serde_json::Value>,
    },

    // 设备配置消息
    DeviceConfig {
        device_id: String,
        config: DeviceConfiguration,
        updated_by: String,
        timestamp: DateTime<Utc>,
    },

    // 设备控制消息
    DeviceControl {
        device_id: String,
        command: DeviceCommand,
        timestamp: DateTime<Utc>,
    },

    // 系统心跳消息
    SystemHeartbeat {
        service: String,
        instance_id: String,
        status: ServiceStatus,
        uptime_seconds: u64,
        timestamp: DateTime<Utc>,
    },

    // 系统状态消息
    SystemStatus {
        service: String,
        status: ServiceStatus,
        message: String,
        details: Option<serde_json::Value>,
        timestamp: DateTime<Utc>,
    },

    // 用户通知消息
    UserNotification {
        user_id: String,
        notification: Notification,
        timestamp: DateTime<Utc>,
    },

    // 广播消息
    Broadcast {
        message_type: String,
        data: serde_json::Value,
        timestamp: DateTime<Utc>,
    },

    // 原始JSON消息
    Raw {
        data: serde_json::Value,
    },
}

// 唤醒原因
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WakeReason {
    VoiceWake,
    ButtonPress,
    Schedule,
    Remote,
    AppTrigger,
    Other(String),
}

// 设备配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceConfiguration {
    pub volume: Option<i32>,
    pub location: Option<String>,
    pub language: Option<String>,
    pub timezone: Option<String>,
    pub wake_word_enabled: Option<bool>,
    pub auto_reply_enabled: Option<bool>,
    pub custom_settings: Option<serde_json::Value>,
}

// 设备控制命令
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DeviceCommand {
    SetVolume { level: i32 },
    SetLocation { location: String },
    Reboot,
    UpdateFirmware { version: String },
    StartSession,
    EndSession,
    PlaySound { sound_type: String },
    Custom { command_type: String, parameters: serde_json::Value },
}

// 服务状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServiceStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Starting,
    Stopping,
    Maintenance,
}

// 通知类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub title: String,
    pub message: String,
    pub level: NotificationLevel,
    pub category: String,
    pub data: Option<serde_json::Value>,
}

// 通知级别
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationLevel {
    Info,
    Warning,
    Error,
    Success,
}

// MQTT 客户端配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttConfig {
    pub broker_host: String,
    pub broker_port: u16,
    pub client_id: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub keep_alive: u64,
    pub clean_session: bool,
    pub max_reconnect_attempts: u32,
    pub reconnect_interval_ms: u64,
}

impl Default for MqttConfig {
    fn default() -> Self {
        Self {
            broker_host: "localhost".to_string(),
            broker_port: 1883,
            client_id: format!("echo-{}", uuid::Uuid::new_v4()),
            username: None,
            password: None,
            keep_alive: 60,
            clean_session: true,
            max_reconnect_attempts: 10,
            reconnect_interval_ms: 5000,
        }
    }
}

// MQTT 错误类型
#[derive(Debug, thiserror::Error)]
pub enum MqttError {
    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Publish error: {0}")]
    Publish(String),

    #[error("Subscribe error: {0}")]
    Subscribe(String),

    #[error("Invalid topic: {0}")]
    InvalidTopic(String),

    #[error("Invalid payload: {0}")]
    InvalidPayload(String),

    #[error("Connection lost")]
    ConnectionLost,

    #[error("Max reconnect attempts reached")]
    MaxReconnectAttemptsReached,
}

// 主题过滤器
#[derive(Debug, Clone)]
pub struct TopicFilter {
    pub topic_pattern: String,
    pub qos: QoS,
}

impl TopicFilter {
    pub fn new(topic_pattern: String, qos: QoS) -> Self {
        Self {
            topic_pattern,
            qos,
        }
    }

    // 常用主题过滤器
    pub fn all_device_status() -> Self {
        Self::new("device/+/status".to_string(), QoS::AtLeastOnce)
    }

    pub fn all_device_wake() -> Self {
        Self::new("device/+/wake".to_string(), QoS::AtLeastOnce)
    }

    pub fn system_status() -> Self {
        Self::new("system/+/status".to_string(), QoS::AtLeastOnce)
    }

    pub fn device_status(device_id: &str) -> Self {
        Self::new(format!("device/{}/status", device_id), QoS::AtLeastOnce)
    }

    pub fn device_config(device_id: &str) -> Self {
        Self::new(format!("device/{}/config", device_id), QoS::AtLeastOnce)
    }

    pub fn all_device_config() -> Self {
        Self::new("device/+/config".to_string(), QoS::AtLeastOnce)
    }

    pub fn all_device_control() -> Self {
        Self::new("device/+/control".to_string(), QoS::AtLeastOnce)
    }

    pub fn device_control(device_id: &str) -> Self {
        Self::new(format!("device/{}/control", device_id), QoS::AtLeastOnce)
    }
}

// 消息构建器
pub struct MqttMessageBuilder;

impl MqttMessageBuilder {
    // 构建设备状态消息
    pub fn device_status(
        device_id: String,
        status: DeviceStatus,
        battery_level: Option<i32>,
        volume: Option<i32>,
        location: Option<String>,
    ) -> MqttMessage {
        let payload = MqttPayload::DeviceStatus {
            device_id: device_id.clone(),
            status,
            battery_level,
            volume,
            location,
            last_seen: Utc::now(),
            metadata: None,
        };

        MqttMessage::new(
            MqttTopic::DeviceStatus(device_id).to_string(),
            payload,
            QoS::AtLeastOnce,
        ).with_retain(true) // 状态消息使用 retain
    }

    // 构建设备配置消息
    pub fn device_config(
        device_id: String,
        config: DeviceConfiguration,
        updated_by: String,
    ) -> MqttMessage {
        let payload = MqttPayload::DeviceConfig {
            device_id: device_id.clone(),
            config,
            updated_by,
            timestamp: Utc::now(),
        };

        MqttMessage::new(
            MqttTopic::DeviceConfig(device_id).to_string(),
            payload,
            QoS::AtLeastOnce,
        )
    }

    // 构建设备控制消息
    pub fn device_control(device_id: String, command: DeviceCommand) -> MqttMessage {
        let payload = MqttPayload::DeviceControl {
            device_id: device_id.clone(),
            command,
            timestamp: Utc::now(),
        };

        MqttMessage::new(
            MqttTopic::DeviceControl(device_id).to_string(),
            payload,
            QoS::AtLeastOnce,
        )
    }

    // 构建系统心跳消息
    pub fn system_heartbeat(
        service: String,
        instance_id: String,
        status: ServiceStatus,
        uptime_seconds: u64,
    ) -> MqttMessage {
        let payload = MqttPayload::SystemHeartbeat {
            service: service.clone(),
            instance_id,
            status,
            uptime_seconds,
            timestamp: Utc::now(),
        };

        MqttMessage::new(
            MqttTopic::SystemHeartbeat(service).to_string(),
            payload,
            QoS::AtMostOnce,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topic_parsing() {
        let topic = "device/dev001/status";
        let parsed = MqttTopic::from_string(topic);
        assert_eq!(parsed, Some(MqttTopic::DeviceStatus("dev001".to_string())));

        let constructed = MqttTopic::DeviceStatus("dev001".to_string()).to_string();
        assert_eq!(constructed, topic);
    }

    #[test]
    fn test_message_builder() {
        let msg = MqttMessageBuilder::device_status(
            "dev001".to_string(),
            DeviceStatus::Online,
            Some(85),
            Some(60),
            Some("living_room".to_string()),
        );

        assert!(matches!(msg.payload, MqttPayload::DeviceStatus { .. }));
        assert_eq!(msg.qos, QoS::AtLeastOnce);
        assert!(msg.retain);
    }
}