/// WebSocket 协议定义
///
/// 兼容 EchoKit Server 的自定义协议（MessagePack + JSON）
/// 用于与 index_zh.html 等 Web 客户端通信

use serde::{Deserialize, Serialize};

/// 客户端命令（来自 Web 客户端）
///
/// 支持 JSON 格式的文本消息
/// 示例：{"event": "StartChat"}
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(tag = "event")]
pub enum ClientCommand {
    /// 开始录制模式会话
    StartRecord,

    /// 开始对话模式会话
    StartChat,

    /// 提交音频数据进行处理
    Submit,

    /// 发送文本输入
    Text { input: String },
}

/// 服务端事件（发送到 Web 客户端）
///
/// 使用 MessagePack 二进制格式编码
/// 对应 EchoKit Server 的 ServerEvent 定义
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum ServerEvent {
    // === 问候消息 ===
    /// 开始发送问候音频
    HelloStart,

    /// 问候音频数据块
    HelloChunk { data: Vec<u8> },

    /// 问候音频结束
    HelloEnd,

    // === 背景音乐 ===
    /// 开始发送背景音乐
    BGStart,

    /// 背景音乐数据块
    BGChunk { data: Vec<u8> },

    /// 背景音乐结束
    BGEnd,

    // === 语音识别结果 ===
    /// ASR（自动语音识别）结果
    ASR { text: String },

    // === 动作指令 ===
    /// 动作指令（用于控制设备行为）
    Action { action: String },

    // === 音频响应 ===
    /// 开始音频响应
    StartAudio { text: String },

    /// 音频数据块（16-bit PCM, 16000Hz, 单声道）
    AudioChunk { data: Vec<u8> },

    /// 音频响应结束
    EndAudio,

    // === 视频响应（预留）===
    /// 开始视频响应
    StartVideo,

    /// 视频响应结束
    EndVideo,

    // === 响应结束标记 ===
    /// 完整响应结束
    EndResponse,
}

impl ClientCommand {
    /// 从 JSON 字符串解析客户端命令
    pub fn from_json(text: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(text)
    }

    /// 判断是否为会话开始命令
    pub fn is_session_start(&self) -> bool {
        matches!(self, ClientCommand::StartChat | ClientCommand::StartRecord)
    }

    /// 判断是否为录制模式
    pub fn is_record_mode(&self) -> bool {
        matches!(self, ClientCommand::StartRecord)
    }
}

impl ServerEvent {
    /// 将事件编码为 MessagePack 二进制格式
    pub fn to_messagepack(&self) -> Result<Vec<u8>, rmp_serde::encode::Error> {
        rmp_serde::to_vec(self)
    }

    /// 从 MessagePack 二进制格式解码事件
    pub fn from_messagepack(data: &[u8]) -> Result<Self, rmp_serde::decode::Error> {
        rmp_serde::from_slice(data)
    }

    /// 判断是否为音频相关事件
    pub fn is_audio_event(&self) -> bool {
        matches!(
            self,
            ServerEvent::StartAudio { .. }
                | ServerEvent::AudioChunk { .. }
                | ServerEvent::EndAudio
        )
    }

    /// 判断是否为控制事件
    pub fn is_control_event(&self) -> bool {
        matches!(
            self,
            ServerEvent::HelloStart
                | ServerEvent::HelloEnd
                | ServerEvent::BGStart
                | ServerEvent::BGEnd
                | ServerEvent::EndResponse
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_command_json_parsing() {
        // 测试 StartChat
        let json = r#"{"event":"StartChat"}"#;
        let cmd = ClientCommand::from_json(json).unwrap();
        assert_eq!(cmd, ClientCommand::StartChat);
        assert!(cmd.is_session_start());
        assert!(!cmd.is_record_mode());

        // 测试 StartRecord
        let json = r#"{"event":"StartRecord"}"#;
        let cmd = ClientCommand::from_json(json).unwrap();
        assert_eq!(cmd, ClientCommand::StartRecord);
        assert!(cmd.is_session_start());
        assert!(cmd.is_record_mode());

        // 测试 Submit
        let json = r#"{"event":"Submit"}"#;
        let cmd = ClientCommand::from_json(json).unwrap();
        assert_eq!(cmd, ClientCommand::Submit);

        // 测试 Text
        let json = r#"{"event":"Text","input":"Hello"}"#;
        let cmd = ClientCommand::from_json(json).unwrap();
        assert_eq!(cmd, ClientCommand::Text { input: "Hello".to_string() });
    }

    #[test]
    fn test_server_event_messagepack_encoding() {
        // 测试 ASR 事件
        let event = ServerEvent::ASR {
            text: "你好世界".to_string(),
        };
        let encoded = event.to_messagepack().unwrap();
        let decoded = ServerEvent::from_messagepack(&encoded).unwrap();
        assert_eq!(event, decoded);

        // 测试 StartAudio 事件
        let event = ServerEvent::StartAudio {
            text: "正在回答".to_string(),
        };
        let encoded = event.to_messagepack().unwrap();
        let decoded = ServerEvent::from_messagepack(&encoded).unwrap();
        assert_eq!(event, decoded);
        assert!(decoded.is_audio_event());

        // 测试 AudioChunk 事件
        let audio_data = vec![1, 2, 3, 4, 5];
        let event = ServerEvent::AudioChunk {
            data: audio_data.clone(),
        };
        let encoded = event.to_messagepack().unwrap();
        let decoded = ServerEvent::from_messagepack(&encoded).unwrap();
        assert_eq!(event, decoded);
        assert!(decoded.is_audio_event());

        // 测试 EndAudio 事件
        let event = ServerEvent::EndAudio;
        let encoded = event.to_messagepack().unwrap();
        let decoded = ServerEvent::from_messagepack(&encoded).unwrap();
        assert_eq!(event, decoded);
        assert!(decoded.is_audio_event());
    }

    #[test]
    fn test_server_event_control_events() {
        let event = ServerEvent::HelloStart;
        assert!(event.is_control_event());
        assert!(!event.is_audio_event());

        let event = ServerEvent::EndResponse;
        assert!(event.is_control_event());
        assert!(!event.is_audio_event());
    }

    #[test]
    fn test_messagepack_compatibility() {
        // 测试与 EchoKit Server 协议的兼容性
        // 确保编码格式一致
        let event = ServerEvent::ASR {
            text: "测试".to_string(),
        };

        let encoded = event.to_messagepack().unwrap();

        // MessagePack 编码应该是紧凑的二进制格式
        assert!(!encoded.is_empty());

        // 验证可以正确解码
        let decoded = ServerEvent::from_messagepack(&encoded).unwrap();
        assert_eq!(event, decoded);
    }
}
