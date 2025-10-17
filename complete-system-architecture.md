# 智能音箱端到端系统完整架构

## 系统架构总览

```mermaid
graph TB
    subgraph "用户界面层"
        WebUI[Web管理界面<br/>TypeScript + React<br/>Ant Design]
    end

    subgraph "接入层"
        Nginx[Nginx反向代理<br/>SSL终止<br/>静态文件服务<br/>负载均衡]
    end

    subgraph "应用服务层 - Rust实现"
        Gateway[API Gateway<br/>Axum框架<br/>RESTful API<br/>WebSocket服务<br/>认证授权]
        Bridge[Bridge服务<br/>Tokio异步运行时<br/>音频流处理<br/>协议转换]
    end

    subgraph "消息中间件"
        MQTT[MQTT Broker<br/>Mosquitto/EMQX<br/>QoS=1保证<br/>设备控制信令]
    end

    subgraph "存储层"
        PG[(PostgreSQL<br/>设备信息<br/>用户数据<br/>会话历史)]
        Redis[(Redis<br/>缓存<br/>会话存储<br/>实时状态)]
    end

    subgraph "AI推理服务层 - Python实现"
        ASR[流式ASR服务<br/>语音识别<br/>gRPC/WebSocket接口]
        DM[Dialogue Manager<br/>对话管理<br/>上下文维护<br/>Rust核心 + Python AI]
        LLM[LLM推理服务<br/>大语言模型<br/>流式生成]
        TTS[TTS合成服务<br/>语音合成<br/>Opus输出]
    end

    subgraph "设备层"
        Device[智能音箱设备<br/>即开即用<br/>无需开发]
    end

    %% 用户交互流
    WebUI -->|HTTPS/WSS| Nginx
    Nginx -->|HTTP/WS| Gateway

    %% API Gateway 交互
    Gateway -->|SQL查询| PG
    Gateway -->|缓存读写| Redis
    Gateway -->|MQTT Pub/Sub| MQTT
    Gateway -->|WebSocket推送| Nginx

    %% 设备交互流
    Device -->|MQTT控制信令<br/>QoS=1| MQTT
    Device -->|UDP音频流<br/>20ms/帧| Bridge
    Device -.->|拉取音频URL<br/>HTTPS| Gateway

    %% MQTT 消息流
    MQTT -->|订阅设备消息| Gateway
    MQTT -->|订阅设备消息| Bridge

    %% Bridge 音频处理流
    Bridge -->|gRPC Streaming<br/>音频流| ASR
    Bridge -->|存储会话数据| PG
    Bridge -->|MQTT发布事件| MQTT

    %% AI 推理链路
    ASR -->|转录文本| DM
    DM -->|Prompt| LLM
    LLM -->|响应文本| DM
    DM -->|待合成文本| TTS
    TTS -->|Opus音频流| Bridge

    %% 样式定义
    classDef frontend fill:#e1f5ff,stroke:#01579b,stroke-width:2px
    classDef rust fill:#ffeaa7,stroke:#d63031,stroke-width:2px
    classDef python fill:#81ecec,stroke:#00b894,stroke-width:2px
    classDef storage fill:#fab1a0,stroke:#e17055,stroke-width:2px
    classDef device fill:#dfe6e9,stroke:#636e72,stroke-width:2px
    classDef middleware fill:#a29bfe,stroke:#6c5ce7,stroke-width:2px

    class WebUI frontend
    class Gateway,Bridge rust
    class ASR,DM,LLM,TTS python
    class PG,Redis storage
    class Device device
    class MQTT,Nginx middleware
```

## 详细交互时序图

### 场景概述

系统涉及四个核心交互场景：
1. **场景1：设备列表查询** - 典型的Web管理操作，展示缓存策略
2. **场景2：设备远程配置** - Web到设备的控制链路，展示MQTT消息流
3. **场景3：语音交互全流程** - 核心业务场景，展示音频处理和AI推理完整链路
4. **场景4：实时状态推送** - WebSocket双向通信，展示实时监控能力

### 时序图

```mermaid
sequenceDiagram
    autonumber
    participant User as 👤 用户<br/>(Web浏览器)
    participant Nginx as 🌐 Nginx
    participant Gateway as 🦀 API Gateway<br/>(Rust/Axum)
    participant PG as 🗄️ PostgreSQL
    participant Redis as 💾 Redis
    participant MQTT as 📡 MQTT Broker
    participant Device as 🔊 智能音箱
    participant Bridge as 🦀 Bridge服务<br/>(Rust/Tokio)
    participant ASR as 🎤 ASR服务<br/>(Python)
    participant DM as 🧠 对话管理<br/>(Rust+Python)
    participant LLM as 🤖 LLM服务<br/>(Python)
    participant TTS as 🗣️ TTS服务<br/>(Python)

    rect rgb(230, 240, 255)
        Note over User,Gateway: 场景1: 用户通过Web界面查看设备列表
        User->>Nginx: HTTPS GET /api/devices
        Nginx->>Gateway: HTTP GET /api/devices<br/>[JWT Token]
        Gateway->>Gateway: 验证JWT Token
        Gateway->>Redis: 检查缓存
        alt 缓存命中
            Redis-->>Gateway: 返回缓存数据
        else 缓存未命中
            Gateway->>PG: SELECT * FROM devices<br/>WHERE owner_id = ?
            PG-->>Gateway: 设备列表数据
            Gateway->>Redis: 更新缓存(TTL=60s)
        end
        Gateway-->>Nginx: JSON响应
        Nginx-->>User: HTTPS响应<br/>[设备列表]
    end

    rect rgb(255, 240, 230)
        Note over User,Device: 场景2: 用户通过Web界面配置设备音量
        User->>Nginx: HTTPS PUT /api/devices/dev123/config<br/>{"volume": 80}
        Nginx->>Gateway: HTTP PUT请求
        Gateway->>Gateway: 验证Token & 权限
        Gateway->>PG: 检查设备归属
        PG-->>Gateway: 设备信息
        Gateway->>PG: UPDATE devices<br/>SET volume = 80
        Gateway->>MQTT: Publish<br/>topic: device/dev123/config<br/>payload: {"volume": 80}<br/>QoS=1
        MQTT->>Device: 推送配置消息
        Device->>Device: 应用配置
        Device->>MQTT: Publish<br/>topic: device/dev123/config/ack
        MQTT->>Gateway: 配置确认
        Gateway->>Redis: 清除设备缓存
        Gateway-->>User: 200 OK<br/>{"success": true}
    end

    rect rgb(240, 255, 240)
        Note over User,TTS: 场景3: 设备唤醒并完成对话交互
        Device->>Device: 本地唤醒词检测
        Device->>MQTT: Publish<br/>topic: device/dev123/wake<br/>payload: {session_id: "s001"}<br/>QoS=1
        MQTT->>Bridge: 订阅唤醒事件
        Bridge->>PG: 创建会话记录
        Bridge->>MQTT: Publish<br/>topic: device/dev123/wake/ack
        MQTT->>Device: 确认唤醒

        Device->>Device: 开始录音<br/>VAD处理<br/>Opus编码

        loop 每20ms音频帧
            Device->>Bridge: UDP数据包<br/>{session_id, seq, audio_data}
        end

        Bridge->>Bridge: 聚合音频帧<br/>抖动缓冲
        Bridge->>ASR: gRPC Streaming<br/>音频流

        ASR-->>Bridge: Partial转录<br/>"今天天气"
        Bridge->>MQTT: Publish<br/>topic: device/dev123/transcript
        MQTT->>Device: 实时转录结果

        ASR-->>Bridge: Final转录<br/>"今天天气怎么样"
        Bridge->>PG: 保存转录文本
        Bridge->>DM: 发送转录文本

        DM->>DM: 整理Prompt<br/>加载上下文
        DM->>LLM: 流式推理请求
        LLM-->>DM: 流式返回<br/>"今天北京晴天..."

        DM->>TTS: 发送完整响应文本
        TTS->>TTS: 语音合成<br/>生成Opus音频
        TTS-->>Bridge: 返回音频流

        Bridge->>PG: 保存响应内容

        alt 方案A: 直接推送
            Bridge->>Device: UDP/TCP推送音频
        else 方案B: URL拉取
            Bridge->>Redis: 存储音频<br/>生成临时URL
            Bridge->>MQTT: Publish<br/>topic: device/dev123/play<br/>payload: {audio_url}
            MQTT->>Device: 播放命令
            Device->>Gateway: HTTPS GET /audio/{token}
            Gateway->>Redis: 获取音频数据
            Gateway-->>Device: 音频流
        end

        Device->>Device: 解码播放音频
        Device->>MQTT: Publish<br/>topic: device/dev123/session_end<br/>payload: {session_id: "s001"}
        MQTT->>Bridge: 会话结束通知
        Bridge->>PG: 更新会话状态<br/>status = 'completed'
    end

    rect rgb(255, 245, 240)
        Note over User,Gateway: 场景4: 实时状态推送(WebSocket)
        User->>Nginx: WSS /ws
        Nginx->>Gateway: WS连接升级
        Gateway->>Gateway: 验证Token
        Gateway->>Gateway: 建立WebSocket连接

        Device->>MQTT: Publish<br/>topic: device/dev123/status<br/>payload: {"online": true, "battery": 85}
        MQTT->>Gateway: 订阅状态更新
        Gateway->>PG: 更新设备状态
        Gateway->>Redis: 更新实时状态
        Gateway->>User: WebSocket推送<br/>{"type": "status_update", "data": {...}}
    end
```

### 场景详细说明

#### 场景1: 用户通过Web界面查看设备列表

**业务场景**：用户登录Web管理界面后，查看自己名下的所有智能音箱设备。

**流程说明**：
1. **用户发起请求**：用户在浏览器中访问设备列表页面，前端发起HTTPS GET请求到 `/api/devices`
2. **Nginx转发**：Nginx作为反向代理，接收HTTPS请求并转发给后端API Gateway
3. **身份验证**：API Gateway首先验证请求中的JWT Token，确认用户身份合法
4. **缓存检查**：验证通过后，Gateway先查询Redis缓存
   - **缓存命中**：如果缓存存在且未过期，直接返回缓存数据（快速路径）
   - **缓存未命中**：查询PostgreSQL数据库获取该用户的设备列表
5. **数据库查询**：执行SQL `SELECT * FROM devices WHERE owner_id = ?`，获取用户的所有设备
6. **更新缓存**：将查询结果写入Redis缓存，设置TTL为60秒
7. **返回响应**：Gateway返回JSON格式的设备列表，经Nginx转发回用户浏览器

**技术要点**：
- 使用Redis缓存减少数据库访问压力
- JWT Token保证API安全性
- 缓存TTL设置平衡实时性和性能

**性能指标**：
- 缓存命中时延迟：< 50ms
- 缓存未命中时延迟：< 200ms

---

#### 场景2: 用户通过Web界面配置设备音量

**业务场景**：用户在Web界面上调整某个智能音箱的音量，系统需要将配置实时推送到设备。

**流程说明**：
1. **用户提交配置**：用户在设备详情页调整音量滑块，前端发起HTTPS PUT请求到 `/api/devices/dev123/config`，Body包含 `{"volume": 80}`
2. **身份和权限验证**：API Gateway验证JWT Token，并检查用户是否有权限配置该设备
3. **设备归属验证**：从PostgreSQL查询设备信息，确认该设备属于当前用户
4. **更新数据库**：执行SQL `UPDATE devices SET volume = 80 WHERE id = 'dev123'`，持久化配置
5. **发布MQTT消息**：Gateway通过MQTT Broker发布消息到主题 `device/dev123/config`，QoS设为1保证消息至少送达一次
6. **设备接收并应用**：智能音箱订阅了该主题，接收到消息后立即调整音量
7. **设备确认**：设备应用配置后，发布确认消息到 `device/dev123/config/ack`
8. **清除缓存**：Gateway接收到确认后，清除Redis中该设备的缓存，确保下次查询获取最新数据
9. **返回成功**：Gateway返回200 OK，前端显示配置成功提示

**技术要点**：
- MQTT QoS=1保证控制命令可靠送达
- 数据库先更新再发送MQTT，保证配置持久化
- 缓存失效策略保证数据一致性
- 设备ACK机制提供可靠反馈

**交互模式**：
- Web → API Gateway → MQTT → Device（控制流）
- Device → MQTT → API Gateway（反馈流）

---

#### 场景3: 设备唤醒并完成对话交互（核心场景）

**业务场景**：用户对智能音箱说"小智小智，今天天气怎么样？"，设备完成从唤醒、识别、推理到语音播放的完整交互。

**流程说明**：

**阶段1：唤醒与会话建立**
1. **本地唤醒检测**：设备通过本地算法检测到唤醒词"小智小智"
2. **发布唤醒事件**：设备通过MQTT发布消息到 `device/dev123/wake`，包含新生成的会话ID `session_id: "s001"`
3. **Bridge响应**：Bridge服务订阅了唤醒事件，接收到消息后在PostgreSQL中创建会话记录
4. **确认唤醒**：Bridge发布ACK消息，设备收到后进入录音模式

**阶段2：音频采集与上传**
5. **开始录音**：设备启动麦克风采集，同时进行VAD（语音活动检测）处理
6. **音频编码**：将音频编码为Opus格式，每20ms生成一个音频帧
7. **UDP传输**：设备通过UDP协议将音频帧发送给Bridge服务，每个数据包包含 `{session_id, seq, audio_data}`
   - 使用UDP而非TCP，优先低延迟而非可靠性
   - 序列号seq用于检测丢包和乱序

**阶段3：语音识别**
8. **音频流聚合**：Bridge接收UDP数据包，进行抖动缓冲处理，平滑网络波动
9. **转发ASR**：Bridge通过gRPC Streaming将音频流转发给ASR服务
10. **实时转录**：ASR返回部分转录结果（Partial Transcript）如"今天天气"，Bridge通过MQTT推送给设备（可选用于回声显示）
11. **最终转录**：ASR返回完整转录结果"今天天气怎么样"
12. **保存转录**：Bridge将最终转录文本保存到PostgreSQL

**阶段4：对话管理与LLM推理**
13. **发送到DM**：Bridge将转录文本发送给Dialogue Manager
14. **整理Prompt**：DM加载用户的历史对话上下文，整理成完整的Prompt
15. **调用LLM**：DM向LLM服务发起流式推理请求
16. **流式返回**：LLM流式生成响应"今天北京晴天，最高气温25度..."
   - 流式生成降低首字延迟，用户体验更好

**阶段5：语音合成**
17. **TTS合成**：DM将完整响应文本发送给TTS服务
18. **生成音频**：TTS将文本合成为Opus格式的音频流
19. **返回Bridge**：TTS将音频流返回给Bridge服务
20. **保存响应**：Bridge将响应内容保存到PostgreSQL

**阶段6：音频下发与播放**
- **方案A（直接推送）**：Bridge通过UDP/TCP直接推送音频数据到设备
- **方案B（URL拉取）**：
  - Bridge将音频临时存储到Redis，生成带有Token的URL
  - 通过MQTT发送播放命令给设备，包含音频URL
  - 设备通过HTTPS从API Gateway拉取音频数据
  - Gateway从Redis获取音频并返回

21. **播放音频**：设备解码Opus音频并通过扬声器播放
22. **会话结束**：播放完成后，设备发布会话结束消息到 `device/dev123/session_end`
23. **更新状态**：Bridge更新PostgreSQL中的会话状态为 `completed`

**技术要点**：
- UDP音频传输优先低延迟，容忍少量丢包
- 流式处理链路：ASR流式识别 + LLM流式生成，降低端到端延迟
- gRPC Streaming保证Bridge到ASR的可靠传输
- 抖动缓冲平滑网络波动
- 会话ID贯穿整个流程，便于追踪和调试

**性能指标**：
- 唤醒到录音：< 100ms
- ASR首字延迟：< 500ms
- LLM首Token：< 2s
- 端到端总延迟：< 3s

**数据流**：
```
设备 → UDP → Bridge → gRPC → ASR → HTTP → DM → HTTP → LLM
                ↓                                      ↓
            PostgreSQL                              TTS
                                                     ↓
设备 ← UDP/HTTPS ← Bridge ← Opus音频流 ← TTS
```

---

#### 场景4: 实时状态推送（WebSocket）

**业务场景**：用户打开Web管理界面后，需要实时看到设备的在线状态、电量等信息变化。

**流程说明**：
1. **建立WebSocket连接**：
   - 用户浏览器发起WSS连接请求到 `/ws`
   - Nginx将WebSocket连接升级请求转发给API Gateway
   - Gateway验证JWT Token确保安全性
   - 验证通过后建立持久化WebSocket连接

2. **设备状态上报**：
   - 智能音箱定期（如每30秒）通过MQTT发布状态消息到 `device/dev123/status`
   - Payload包含：`{"online": true, "battery": 85, "volume": 80, "temperature": 35}`

3. **Gateway订阅与处理**：
   - API Gateway订阅了所有设备的状态主题 `device/+/status`
   - 接收到设备状态消息后：
     - 更新PostgreSQL中的设备状态（持久化）
     - 更新Redis中的实时状态（快速查询）
     - 判断哪些WebSocket连接需要接收该设备状态（权限过滤）

4. **实时推送**：
   - Gateway通过WebSocket推送消息给所有有权限的在线用户
   - 推送格式：`{"type": "status_update", "device_id": "dev123", "data": {"online": true, "battery": 85}}`
   - 前端接收后实时更新UI，用户无需刷新页面

**技术要点**：
- WebSocket保持长连接，避免HTTP轮询开销
- MQTT订阅通配符 `device/+/status` 监听所有设备
- 权限过滤：只推送用户有权查看的设备状态
- 双重存储：PostgreSQL持久化 + Redis实时缓存
- 心跳机制：检测WebSocket连接存活

**优势**：
- ✅ 实时性高：设备状态变化立即推送，延迟< 1s
- ✅ 服务器资源友好：避免大量HTTP轮询请求
- ✅ 用户体验好：界面实时更新，无需手动刷新

**扩展应用**：
- 会话进度通知（正在识别、正在思考、正在合成）
- 设备异常告警（离线、低电量、高温）
- 系统通知（固件更新可用）

---

### 场景对比总结

| 场景 | 通信协议 | 数据流向 | 核心目标 | 延迟要求 |
|------|---------|---------|---------|---------|
| 场景1：查询设备 | HTTPS | Web → Gateway → DB/Cache | 快速查询 | < 200ms |
| 场景2：配置设备 | HTTPS + MQTT | Web → Gateway → MQTT → Device | 可靠控制 | < 1s |
| 场景3：语音交互 | UDP + gRPC + MQTT | Device → Bridge → AI → Device | 低延迟交互 | < 3s |
| 场景4：状态推送 | MQTT + WebSocket | Device → MQTT → Gateway → Web | 实时监控 | < 1s |

**设计理念**：
- **查询场景**：缓存优先，提升响应速度
- **控制场景**：MQTT QoS保证，确保命令送达
- **音频场景**：UDP + 流式处理，极致低延迟
- **推送场景**：WebSocket长连接，减少开销

## 技术架构分层视图

```mermaid
graph TB
    subgraph "Layer 1: 设备层"
        D1[智能音箱硬件]
        D2[即开即用 - 无需开发]
        D3[唤醒词检测]
        D4[音频采集与播放]
        D5[MQTT客户端]
    end

    subgraph "Layer 2: 接入与通信层"
        N1[Nginx反向代理]
        N2[SSL/TLS终止]
        N3[静态文件服务]

        M1[MQTT Broker]
        M2[消息路由]
        M3[QoS保证]
    end

    subgraph "Layer 3: 应用服务层 - Rust"
        G1[API Gateway - Axum]
        G2[RESTful API]
        G3[WebSocket服务]
        G4[JWT认证]
        G5[权限控制 RBAC]

        B1[Bridge服务 - Tokio]
        B2[UDP音频接收]
        B3[音频流聚合]
        B4[抖动缓冲]
        B5[gRPC客户端]
    end

    subgraph "Layer 4: 数据存储层"
        DB1[PostgreSQL]
        DB2[设备管理表]
        DB3[用户认证表]
        DB4[会话历史表]

        C1[Redis]
        C2[缓存层]
        C3[会话存储]
        C4[实时状态]
    end

    subgraph "Layer 5: AI推理服务层 - Python"
        A1[ASR服务]
        A2[流式语音识别]

        DM1[Dialogue Manager]
        DM2[核心逻辑 Rust]
        DM3[AI调用 Python]

        L1[LLM服务]
        L2[流式推理]

        T1[TTS服务]
        T2[语音合成]
    end

    subgraph "Layer 6: 前端展示层"
        F1[React/Vue + TypeScript]
        F2[设备管理界面]
        F3[会话历史查询]
        F4[实时监控面板]
        F5[配置管理]
    end

    %% 层级关系
    D1 -.-> N1
    D1 -.-> M1
    D1 -.-> B1

    F1 --> N1
    N1 --> G1
    M1 --> G1
    M1 --> B1

    G1 --> DB1
    G1 --> C1
    B1 --> DB1
    B1 --> A1

    A1 --> DM1
    DM1 --> L1
    DM1 --> T1
    T1 --> B1
```

## 数据流向图

```mermaid
flowchart LR
    subgraph "音频上行链路 - 低延迟优先"
        DEV1[设备采集音频] -->|UDP<br/>20ms/帧| BR1[Bridge聚合]
        BR1 -->|gRPC Stream| ASR1[ASR识别]
        ASR1 -->|转录文本| DM1[对话管理]
        DM1 -->|Prompt| LLM1[LLM推理]
    end

    subgraph "音频下行链路"
        LLM1 -->|响应文本| TTS1[TTS合成]
        TTS1 -->|Opus音频| BR2[Bridge分发]
        BR2 -->|UDP推送<br/>或URL拉取| DEV2[设备播放]
    end

    subgraph "控制信令链路 - 可靠性优先"
        WEB1[Web界面] -->|HTTPS| GW1[API Gateway]
        GW1 -->|MQTT QoS=1| MQ1[MQTT Broker]
        MQ1 -->|订阅| DEV3[设备接收]
        DEV3 -->|MQTT QoS=1| MQ2[MQTT Broker]
        MQ2 -->|订阅| GW2[API Gateway]
        GW2 -->|WebSocket| WEB2[Web实时推送]
    end

    subgraph "数据持久化"
        GW1 -->|SQL| DB1[(PostgreSQL)]
        BR1 -->|SQL| DB1
        GW1 -->|Cache| RD1[(Redis)]
    end

    style DEV1 fill:#dfe6e9
    style DEV2 fill:#dfe6e9
    style DEV3 fill:#dfe6e9
    style BR1 fill:#ffeaa7
    style BR2 fill:#ffeaa7
    style GW1 fill:#ffeaa7
    style GW2 fill:#ffeaa7
    style ASR1 fill:#81ecec
    style DM1 fill:#81ecec
    style LLM1 fill:#81ecec
    style TTS1 fill:#81ecec
    style WEB1 fill:#e1f5ff
    style WEB2 fill:#e1f5ff
```

## 部署架构图

```mermaid
graph TB
    subgraph "用户侧"
        Browser[浏览器<br/>Chrome/Safari/Firefox]
        Speaker[智能音箱设备<br/>WiFi连接]
    end

    subgraph "云端部署 - Kubernetes集群"
        subgraph "边缘节点 Pod"
            Nginx[Nginx Ingress<br/>负载均衡<br/>SSL终止]
        end

        subgraph "应用层 Pods - Rust"
            GW1[API Gateway<br/>副本1]

            BR1[Bridge服务<br/>副本1]
        end

        subgraph "中间件 Pods"
            MQTT1[MQTT Broker<br/>主节点]
            MQTT2[MQTT Broker<br/>从节点]
        end

        subgraph "存储层 StatefulSet"
            PG1[(PostgreSQL<br/>主)]
            PG2[(PostgreSQL<br/>从)]
            Redis1[(Redis<br/>主)]
            Redis2[(Redis<br/>从)]
        end

        subgraph "AI服务层 Pods - Python"
            ASR1[ASR服务<br/>GPU节点]

            LLM1[LLM服务<br/>GPU节点]

            TTS1[TTS服务<br/>GPU节点]

            DM1[对话管理<br/>副本1]
        end

        subgraph "监控与日志"
            Prom[Prometheus<br/>指标收集]
            Graf[Grafana<br/>可视化]
            ELK[ELK Stack<br/>日志分析]
        end
    end

    %% 连接关系
    Browser -->|HTTPS/WSS| Nginx
    Speaker -->|MQTT/UDP| Nginx

    Nginx --> GW1
    Nginx --> BR1

    GW1 --> MQTT1
    GW1 --> PG1
    GW1 --> Redis1

    BR1 --> MQTT1
    BR1 --> ASR1
    BR1 --> PG1

    ASR1 --> DM1
    DM1 --> LLM1
    DM1 --> TTS1

    MQTT1 -.->|复制| MQTT2
    PG1 -.->|主从复制| PG2
    Redis1 -.->|主从复制| Redis2

    GW1 -.->|指标| Prom
    BR1 -.->|指标| Prom
    Prom --> Graf

    GW1 -.->|日志| ELK
    BR1 -.->|日志| ELK

    style Browser fill:#e1f5ff
    style Speaker fill:#dfe6e9
    style GW1 fill:#ffeaa7
    style BR1 fill:#ffeaa7
    style ASR1 fill:#81ecec
    style LLM1 fill:#81ecec
    style TTS1 fill:#81ecec
    style DM1 fill:#81ecec
```

## 核心技术栈总结

### 前端层
| 组件 | 技术选型 | 说明 |
|------|---------|------|
| UI框架 | React 18 + TypeScript | 类型安全,生态丰富 |
| 状态管理 | Zustand / Redux Toolkit | 轻量级状态管理 |
| UI组件库 | Ant Design | 企业级组件库 |
| 数据请求 | TanStack Query | 数据获取与缓存 |
| 实时通信 | WebSocket API | 实时状态推送 |
| 构建工具 | Vite | 快速构建 |

### 后端层 (Rust)
| 组件 | 技术选型 | 说明 |
|------|---------|------|
| API Gateway | Axum | 现代化Web框架 |
| Bridge服务 | Tokio | 异步运行时 |
| gRPC客户端 | Tonic | gRPC框架 |
| MQTT客户端 | rumqttc | MQTT 3.1.1/5.0 |
| 数据库访问 | sqlx | 异步SQL |
| 音频编解码 | opus | Opus编解码 |
| 认证 | jsonwebtoken | JWT |

### 中间件层
| 组件 | 技术选型 | 说明 |
|------|---------|------|
| 反向代理 | Nginx | SSL终止,负载均衡 |
| 消息代理 | Mosquitto/EMQX | MQTT Broker |
| 关系数据库 | PostgreSQL 15+ | 事务支持 |
| 缓存 | Redis 7+ | 高性能缓存 |

### AI服务层 (Python)
| 组件 | 技术选型 | 说明 |
|------|---------|------|
| ASR | FastAPI + 模型 | 流式语音识别 |
| LLM | vLLM / TGI | 流式推理 |
| TTS | FastAPI + 模型 | 语音合成 |
| 对话管理 | Rust核心 + Python | 混合实现 |

### 基础设施
| 组件 | 技术选型 | 说明 |
|------|---------|------|
| 容器编排 | Kubernetes | 微服务部署 |
| 容器运行时 | Docker | 容器化 |
| 监控 | Prometheus + Grafana | 指标监控 |
| 日志 | ELK Stack | 日志分析 |
| CI/CD | GitHub Actions | 自动化部署 |

## 关键性能指标

### 延迟要求
- **唤醒响应**: < 100ms
- **音频上传**: 20ms/帧 (实时)
- **ASR识别**: < 500ms (首字延迟)
- **LLM推理**: < 2s (首Token)
- **TTS合成**: < 1s
- **端到端**: < 3s (从说话到播放)

### 吞吐量要求
- **并发设备**: 10,000+ 设备同时在线
- **API QPS**: 5,000+ 请求/秒
- **音频流**: 1,000+ 并发流
- **WebSocket连接**: 10,000+ 并发连接

### 可用性要求
- **系统可用性**: 99.9% (年停机时间 < 8.76小时)
- **数据持久性**: 99.999%
- **消息送达**: QoS=1 至少一次

## 安全架构

```mermaid
graph TB
    subgraph "安全防护体系"
        subgraph "网络层安全"
            FW[防火墙<br/>端口控制]
            DDoS[DDoS防护<br/>流量清洗]
            SSL[SSL/TLS加密<br/>证书管理]
        end

        subgraph "应用层安全"
            Auth[JWT认证<br/>Token管理]
            RBAC[RBAC权限控制<br/>角色管理]
            Valid[输入验证<br/>SQL注入防护]
            Rate[限流<br/>Rate Limiting]
        end

        subgraph "数据层安全"
            Encrypt[数据加密<br/>AES-256]
            Backup[定期备份<br/>异地容灾]
            Audit[审计日志<br/>操作追踪]
        end

        subgraph "设备层安全"
            DevAuth[设备认证<br/>证书/密钥]
            MQTTAuth[MQTT认证<br/>用户名密码]
        end
    end

    FW --> SSL
    SSL --> Auth
    Auth --> RBAC
    RBAC --> Valid
    Valid --> Rate

    Rate --> Encrypt
    Encrypt --> Backup
    Backup --> Audit

    DevAuth --> MQTTAuth
    MQTTAuth --> SSL
```

## 扩展性设计

### 水平扩展能力
- ✅ **API Gateway**: 无状态设计,可任意扩展Pod数量
- ✅ **Bridge服务**: 通过会话ID分片,独立扩展
- ✅ **AI服务**: GPU节点池,按需扩展
- ✅ **MQTT Broker**: 支持集群模式
- ✅ **数据库**: 读写分离,从库扩展

### 垂直扩展能力
- ✅ **增加CPU/内存**: 优化单实例性能
- ✅ **GPU升级**: AI推理加速
- ✅ **存储扩展**: 增加磁盘容量

## 监控与可观测性

```mermaid
graph LR
    subgraph "监控体系"
        M1[指标监控<br/>Prometheus]
        M2[日志聚合<br/>ELK Stack]
        M3[链路追踪<br/>Jaeger/Tempo]
        M4[告警通知<br/>AlertManager]
    end

    subgraph "可视化"
        D1[Grafana<br/>实时大盘]
        D2[Kibana<br/>日志查询]
    end

    subgraph "数据源"
        S1[应用指标<br/>延迟/QPS/错误率]
        S2[系统指标<br/>CPU/内存/网络]
        S3[业务指标<br/>设备数/会话数]
        S4[日志流<br/>结构化日志]
    end

    S1 & S2 & S3 --> M1
    S4 --> M2
    M1 --> M3
    M1 --> D1
    M2 --> D2
    M1 --> M4
```

## 关键设计原则

### 1. 性能优先
- 音频链路走UDP,牺牲可靠性换取低延迟
- 控制信令走MQTT,保证可靠送达
- 异步非阻塞I/O (Tokio)
- 流式处理降低端到端延迟

### 2. 安全可靠
- JWT Token认证
- RBAC权限控制
- MQTT QoS=1保证消息送达
- 数据库事务保证一致性

### 3. 可扩展性
- 微服务架构,组件独立扩展
- 无状态设计,水平扩展
- 服务发现与负载均衡
- 缓存层减轻数据库压力

### 4. 可维护性
- 统一的日志格式
- 完善的监控告警
- 清晰的代码结构
- 完整的API文档

### 5. 用户体验
- 实时反馈 (WebSocket)
- 友好的Web界面
- 低延迟语音交互
- 可靠的消息送达

---

**文档版本**: v1.0
**创建日期**: 2025-10-17
**技术栈**: Rust + TypeScript + Python
**适用场景**: 智能音箱端到端系统设计
