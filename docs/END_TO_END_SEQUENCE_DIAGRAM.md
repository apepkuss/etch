# 智能音箱端到端场景时序图

## 完整交互流程

```mermaid
sequenceDiagram
    participant Device as 智能音箱设备
    participant MQTT as MQTT Broker
    participant Bridge as Bridge服务
    participant ASR as 流式ASR服务
    participant DM as Dialogue Manager
    participant LLM as LLM推理服务
    participant TTS as TTS合成服务

    Note over Device: 1. 唤醒 & 触发
    Device->>Device: 本地检测唤醒词
    Device->>MQTT: 发布消息<br/>topic: device/{id}/wake<br/>payload: {session_id}<br/>QoS=1

    Note over Device,Bridge: 2. 开始录音 & 传输音频
    Device->>Device: 开始采集音频<br/>VAD/segment处理<br/>编码为Opus
    loop 每20ms音频帧
        Device->>Bridge: UDP发送音频帧<br/>{session_id, seq, audio_data}
    end

    Note over Bridge,ASR: 3. Bridge接收 & 转发给ASR
    Bridge->>Bridge: UDP Listener聚合帧<br/>抖动缓冲处理
    Bridge->>ASR: gRPC Streaming/WebSocket<br/>转发音频流

    Note over ASR,Device: 4. ASR返回实时转录
    ASR-->>Bridge: 返回partial transcripts<br/>(实时中间结果)
    Bridge-->>MQTT: 发布转录事件<br/>topic: device/{id}/transcript
    MQTT-->>Device: 接收partial结果<br/>(可选回声提示)
    ASR-->>Bridge: 返回final transcript<br/>(最终转录结果)
    Bridge-->>MQTT: 发布最终转录
    MQTT-->>Device: 接收最终转录

    Note over DM,LLM: 5. LLM推理
    Bridge->>DM: 发送转录文本
    DM->>DM: 整理为prompt<br/>附加上下文
    DM->>LLM: 调用流式推理API<br/>streaming inference
    LLM-->>DM: 流式返回响应文本
    DM->>DM: 处理响应内容

    Note over TTS,Device: 6. TTS生成 & 返回音频
    DM->>TTS: 发送待合成文本
    TTS->>TTS: 合成音频(Opus)
    TTS-->>Bridge: 返回音频流

    alt 方案A: UDP/TCP直传
        Bridge-->>Device: UDP/TCP传输音频
    else 方案B: 云端拉取
        Bridge->>MQTT: 发布播放控制消息<br/>topic: device/{id}/play<br/>payload: {audio_url}
        MQTT-->>Device: 接收播放命令
        Device->>Bridge: HTTP/HTTPS拉取音频
        Bridge-->>Device: 返回音频数据
    end

    Note over Device: 7. 播放 & 会话结束
    Device->>Device: 解码并播放音频
    Device->>Device: 播放完成
    Device->>MQTT: 发布会话结束<br/>topic: device/{id}/session_end<br/>payload: {session_id}
```

## 关键技术点说明

### 通信协议选择
- **MQTT**: 用于控制信令（QoS=1保证至少一次送达）
- **UDP**: 用于实时音频传输（低延迟优先）
- **gRPC Streaming/WebSocket**: 用于Bridge到ASR的可靠流式传输

### 数据格式
- **音频编码**: Opus（20ms帧）
- **消息载荷**: JSON格式，包含session_id用于会话跟踪

### 性能优化
- VAD（Voice Activity Detection）：减少静音传输
- 抖动缓冲：平滑网络波动
- 流式处理：ASR、LLM均支持流式，降低端到端延迟

### QoS保证
- MQTT QoS=1：确保关键控制消息送达
- UDP音频传输：容忍少量丢包，优先低延迟
- 序列号（seq）：用于检测丢包和乱序
