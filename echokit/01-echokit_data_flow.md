# EchoKit 数据流图

## 整体架构数据流

```mermaid
flowchart TB
    subgraph Device["ESP32-S3 设备"]
        MIC[麦克风]
        SPK[扬声器]
        FW[固件<br/>Rust]
    end

    subgraph Server["EchoKit Server"]
        WS[WebSocket 服务<br/>ws.rs]
        PROC[音频处理<br/>PCM ↔ WAV]
    end

    subgraph AI["AI 服务集群"]
        VAD_S[VAD 服务器<br/>Silero VAD]
        ASR_S[ASR 服务器<br/>Whisper]
        LLM_S[LLM 服务器<br/>GPT/Gemini]
        TTS_S[TTS 服务器<br/>GPT-SoVITS/Fish]
    end

    MIC -->|1.音频采集<br/>16kHz PCM| FW
    FW -->|2.WebSocket Binary<br/>PCM 数据| WS
    WS -->|3a. HTTP POST<br/>WAV 文件| VAD_S
    WS -->|3b. HTTP POST<br/>WAV 文件| ASR_S
    VAD_S -->|4a. JSON 响应<br/>语音段时间戳| WS
    ASR_S -->|4b. JSON 响应<br/>识别文本| WS
    WS -->|5.HTTP POST<br/>JSON 消息| LLM_S
    LLM_S -->|6.JSON 响应<br/>回复文本| WS
    WS -->|7.HTTP POST<br/>文本| TTS_S
    TTS_S -->|8.WAV/PCM<br/>音频数据| WS
    WS -->|9.WebSocket Binary<br/>PCM 数据| FW
    FW -->|10.音频播放| SPK

    style Device fill:#e1f5ff
    style Server fill:#fff4e1
    style AI fill:#f0e1ff
```

## 详细数据流（Standard 配置）

```mermaid
sequenceDiagram
    participant ESP as ESP32-S3 设备
    participant WS as WebSocket 服务器
    participant VAD as VAD 服务 (HTTP)
    participant ASR as ASR 服务 (HTTP)
    participant LLM as LLM 服务 (HTTP)
    participant TTS as TTS 服务 (HTTP)

    Note over ESP,TTS: 1️⃣ 语音输入阶段
    ESP->>WS: WebSocket Binary<br/>PCM 数据 (16kHz, 16-bit)
    Note over WS: 收集 PCM → 转换为 WAV

    Note over WS,VAD: 2️⃣ 语音检测阶段
    WS->>VAD: HTTP POST<br/>multipart/form-data<br/>WAV 文件
    VAD-->>WS: JSON Response<br/>timestamps: [...]

    Note over WS,ASR: 3️⃣ 语音识别阶段
    WS->>ASR: HTTP POST<br/>multipart/form-data<br/>WAV 文件
    ASR-->>WS: JSON Response<br/>text: 识别文本

    Note over WS,LLM: 4️⃣ 大模型推理阶段
    WS->>LLM: HTTP POST<br/>application/json<br/>messages: [...]
    LLM-->>WS: JSON Response (streaming)<br/>choices: [delta: content: ...]

    Note over WS,TTS: 5️⃣ 语音合成阶段
    WS->>TTS: HTTP POST<br/>application/json<br/>input: 回复文本
    TTS-->>WS: WAV 音频数据<br/>(32kHz, 16-bit)

    Note over WS,ESP: 6️⃣ 音频下发阶段
    Note over WS: 重采样 32kHz → 16kHz<br/>分块 0.5秒
    WS->>ESP: WebSocket Binary<br/>PCM chunks (16kHz)
    Note over ESP: 播放音频
```

## Gemini Live 配置数据流

```mermaid
sequenceDiagram
    participant ESP as ESP32-S3 设备
    participant WS as WebSocket 服务器
    participant Gemini as Gemini Live API (WebSocket)

    Note over ESP,Gemini: 🔄 实时双向流式通信

    WS->>Gemini: WebSocket Text<br/>Setup 配置
    Gemini-->>WS: setupComplete

    Note over ESP,WS: 用户说话
    ESP->>WS: WebSocket Binary<br/>PCM 音频流
    Note over WS: 收集完整音频
    WS->>Gemini: WebSocket Binary<br/>RealtimeAudio<br/>data: PCM, mime: audio/pcm rate=16000

    Note over Gemini: 集成 VAD + ASR + LLM + TTS

    Gemini-->>WS: InputTranscription<br/>text: 识别结果
    WS->>ESP: MessagePack Binary<br/>ASR 事件

    Gemini-->>WS: ModelTurn (streaming)<br/>文本 + 音频
    Note over WS: 提取音频数据<br/>分块 0.5秒
    WS->>ESP: WebSocket Binary<br/>PCM chunks

    Gemini-->>WS: TurnComplete
    WS->>ESP: EndAudio 事件
```

## Paraformer V2 实时 ASR 数据流

```mermaid
sequenceDiagram
    participant ESP as ESP32-S3 设备
    participant WS as EchoKit WebSocket
    participant PF as Paraformer V2 WebSocket

    Note over WS,PF: 🔌 建立连接
    WS->>PF: WebSocket Upgrade<br/>wss://dashscope.aliyuncs.com<br/>Bearer Token

    Note over WS,PF: 🚀 启动任务
    WS->>PF: WebSocket Text<br/>header: action: run-task<br/>payload: model: paraformer-realtime-v2
    PF-->>WS: task-started

    Note over ESP,WS: 🎤 实时音频流
    ESP->>WS: WebSocket Binary<br/>PCM 数据
    WS->>PF: WebSocket Binary<br/>PCM 数据 (实时转发)

    Note over PF: 实时识别处理

    PF-->>WS: WebSocket Text<br/>sentence: text: ..., sentence_end: false
    PF-->>WS: WebSocket Text<br/>sentence: text: 完整句子, sentence_end: true

    Note over WS,PF: 🏁 结束任务
    WS->>PF: WebSocket Text<br/>header: action: finish-task
    PF-->>WS: task-finished
```

## 数据格式详解

### 1. WebSocket 消息格式（设备 ↔ 服务器）

```mermaid
flowchart LR
    subgraph Client["客户端消息 (设备 → 服务器)"]
        C1["Binary Message<br/>PCM 音频数据<br/>16kHz, 16-bit, LE"]
        C2["Text Message<br/>JSON 事件<br/>event: StartChat"]
    end

    subgraph Server["服务器消息 (服务器 → 设备)"]
        S1["Binary Message<br/>MessagePack 序列化<br/>ServerEvent 枚举"]
        S2["ASR 事件<br/>text: ..."]
        S3["Audio 事件<br/>data: PCM"]
        S4["StartAudio 事件<br/>text: ..."]
    end

    C1 -.->|音频流| S1
    C2 -.->|控制| S1
    S1 -->|解析| S2
    S1 -->|解析| S3
    S1 -->|解析| S4
```

### 2. HTTP 请求格式

```mermaid
flowchart TB
    subgraph VAD["VAD 请求"]
        VAD1["POST /v1/audio/vad<br/>Content-Type: multipart/form-data<br/>---<br/>audio: WAV 文件"]
        VAD2["Response:<br/>timestamps: [start: 0.5, end: 2.3]"]
        VAD1 --> VAD2
    end

    subgraph ASR["ASR 请求 (Whisper)"]
        ASR1["POST /v1/audio/transcriptions<br/>Content-Type: multipart/form-data<br/>---<br/>file: WAV 文件<br/>language: zh<br/>model: whisper-large-v3"]
        ASR2["Response:<br/>text: 识别的文本内容"]
        ASR1 --> ASR2
    end

    subgraph LLM["LLM 请求"]
        LLM1["POST /v1/chat/completions<br/>Content-Type: application/json<br/>---<br/>messages: [role: user, content: ...]"]
        LLM2["Response (streaming):<br/>data: choices: [delta: content: ...]<br/>data: [DONE]"]
        LLM1 --> LLM2
    end

    subgraph TTS["TTS 请求"]
        TTS1["POST /v1/audio/speech<br/>Content-Type: application/json<br/>---<br/>input: 要合成的文本, speaker: ad"]
        TTS2["Response:<br/>WAV 音频文件<br/>(32kHz, 16-bit)"]
        TTS1 --> TTS2
    end
```

## 音频处理流程

```mermaid
flowchart LR
    subgraph Input["📥 音频输入"]
        I1[ESP32-S3<br/>采集]
        I2[16kHz<br/>16-bit<br/>PCM<br/>Mono]
    end

    subgraph Process["⚙️ 服务器处理"]
        P1[收集 PCM]
        P2[转换为 WAV]
        P3[发送给 VAD/ASR]
        P4[接收 TTS 音频<br/>32kHz WAV]
        P5[重采样<br/>32kHz → 16kHz]
        P6[分块<br/>0.5 秒/块]
    end

    subgraph Output["📤 音频输出"]
        O1[16kHz<br/>16-bit<br/>PCM<br/>Chunks]
        O2[ESP32-S3<br/>播放]
    end

    I1 --> I2
    I2 -->|WebSocket<br/>Binary| P1
    P1 --> P2
    P2 --> P3
    P4 --> P5
    P5 --> P6
    P6 -->|WebSocket<br/>Binary| O1
    O1 --> O2
```

## 协议栈对比

| 组件 | 通信协议 | 数据格式 | 音频格式 | 特点 |
|------|---------|---------|---------|------|
| **设备 ↔ EchoKit** | WebSocket | Binary (PCM) + MessagePack | 16kHz, 16-bit PCM | 双向实时 |
| **EchoKit ↔ VAD** | HTTP POST | multipart/form-data | 16kHz WAV | 批处理 |
| **EchoKit ↔ ASR (Whisper)** | HTTP POST | multipart/form-data | 16kHz WAV | 批处理 |
| **EchoKit ↔ ASR (Paraformer)** | WebSocket | Binary (PCM) | 16kHz PCM | 实时流式 |
| **EchoKit ↔ LLM** | HTTP POST | JSON | 无音频 | 流式响应 |
| **EchoKit ↔ TTS** | HTTP POST | JSON | 32kHz WAV | 批处理/流式 |
| **EchoKit ↔ Gemini Live** | WebSocket | 自定义协议 | 16kHz PCM | 全集成实时 |

## 消息序列化

```mermaid
flowchart TB
    subgraph Serialization["MessagePack 序列化流程"]
        E1[ServerEvent 枚举]
        E2{事件类型}
        E3["ASR 事件<br/>text: String"]
        E4["Audio 事件<br/>data: Vec u8"]
        E5["StartAudio 事件<br/>text: String"]

        M1[MessagePack<br/>二进制编码]
        M2[WebSocket<br/>Binary Message]

        E1 --> E2
        E2 -->|ASR| E3
        E2 -->|Audio| E4
        E2 -->|StartAudio| E5
        E3 --> M1
        E4 --> M1
        E5 --> M1
        M1 --> M2
    end

    subgraph Device["ESP32-S3 解析"]
        D1[接收 Binary]
        D2[MessagePack<br/>反序列化]
        D3[处理事件]

        D1 --> D2
        D2 --> D3
    end

    M2 -.->|网络传输| D1
```

## 总结

### 🎯 核心数据流特点

1. **设备通信**：WebSocket + MessagePack 二进制序列化
2. **VAD 服务**：HTTP POST + multipart/form-data (批处理)
3. **ASR 服务**：
   - Whisper: HTTP POST + multipart/form-data (批处理)
   - Paraformer V2: WebSocket + 二进制流 (实时)
4. **LLM 服务**：HTTP POST + JSON (流式响应)
5. **TTS 服务**：HTTP POST + JSON → WAV 音频
6. **音频格式**：统一使用 16kHz, 16-bit, PCM, Mono

### 🔄 两种架构模式

**标准模式（Stable）：**

- VAD (HTTP) → ASR (HTTP) → LLM (HTTP) → TTS (HTTP)
- 每个服务独立调用，灵活可配置

**Gemini Live 模式：**

- 单一 WebSocket 连接完成所有功能
- VAD + ASR + LLM + TTS 全部集成
- 延迟更低，但灵活性降低
