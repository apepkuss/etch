# Bridge WebUI 语音交互演示指南

## 🎯 概述

本指南详细说明如何使用 Bridge 内置测试前端（`bridge_webui.html`）演示完整的语音交互功能，包括对话模式和录制模式。

---

## 📋 前提条件

### 必需条件
- ✅ Bridge 服务已编译（或准备好编译）
- ✅ 浏览器支持麦克风访问（Chrome/Edge/Safari 推荐）
- ✅ 网络连接正常（对话模式需要访问 EchoKit Server）

### 可选条件
- 🔧 配置自定义 EchoKit Server URL
- 🎧 耳机或扬声器（接收语音回复）

---

## 🚀 快速启动

### 方式 1：使用自动化脚本（推荐）⭐

```bash
cd /Volumes/Dev/secondstate/me/etch/bridge

# 运行启动脚本
./start_test.sh
```

**脚本首先会显示服务依赖清单：**

```
📦 测试服务依赖清单：
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
必需服务：
  🟢 Bridge WebSocket    端口: 10031 (HTTP + WebSocket)
     └─ 提供 WebSocket 通信和静态文件服务

可选服务（用于设备通信测试）：
  🟡 Bridge UDP Server   端口: 8083 (默认，可配置)
     └─ 接收设备音频数据
  🟡 MQTT Broker         端口: 1883 (mqtt:1883)
     └─ 设备控制和状态同步

外部依赖（对话模式测试）：
  🔵 EchoKit Server      URL: wss://indie.echokit.dev/ws/{visitor-id}
     └─ 提供语音识别和对话服务

💡 提示：
   - WebSocket 测试只需要 Bridge WebSocket (10031) 服务
   - UDP/MQTT 错误不影响 WebSocket 测试
   - EchoKit Server 仅对话模式需要，连接测试可忽略
```

**然后是交互式配置提示：**

1. **是否重新编译 Bridge？**
   ```
   🔨 编译 (y/N):
   ```
   - 首次运行或代码有更新：输入 `y`
   - 已编译且代码无变化：直接回车（默认 `n`）

2. **UDP 端口配置（如有冲突）**
   ```
   ⚠️  警告：默认 UDP 端口 8083 已被占用
      请输入替代 UDP 端口 [18083]:
   ```
   - 默认使用 8083
   - 如被占用，脚本会提示输入其他端口
   - 推荐范围：1024-65535
   - **注意**：WebSocket 测试不依赖 UDP 端口

3. **配置 EchoKit Server URL**
   ```
   🔧 配置 EchoKit Server URL（对话模式测试需要）
      格式: wss://indie.echokit.dev/ws/{your-visitor-id}
      EchoKit URL [wss://indie.echokit.dev/ws/ci-test-visitor]:
   ```
   - 使用默认测试 ID：直接回车
   - 使用自定义 Visitor ID：输入完整 URL

4. **是否自动打开浏览器？**
   ```
   🌐 是否自动打开浏览器？(Y/n):
   ```
   - 直接回车或输入 `y`：自动打开浏览器
   - 输入 `n`：手动打开

**启动成功后：**
- ✅ Bridge 服务运行在 `http://localhost:10031`
- ✅ 浏览器自动打开测试页面
- ✅ 日志文件：`logs/bridge.log`

---

### 方式 2：手动启动

```bash
cd /Volumes/Dev/secondstate/me/etch/bridge

# 1. 配置 EchoKit Server（对话模式需要）
export ECHOKIT_WEBSOCKET_URL="wss://indie.echokit.dev/ws/ci-test-visitor"

# 2. 编译（首次或代码有更新）
cargo build --release

# 3. 启动 Bridge 服务
cargo run --release

# 4. 打开浏览器
open http://localhost:10031/bridge_webui.html
```

**预期输出：**
```
INFO Bridge service starting...
INFO WebSocket server listening on: 0.0.0.0:10031
INFO Static files served from: resources/
INFO EchoKit connection established successfully
```

**可以安全忽略的警告：**
```
ERROR echo_bridge::mqtt_client: MQTT connection error: I/O: failed to lookup...
ERROR echo_bridge::mqtt_client: MQTT event loop terminated with error...
```
- 这些是 MQTT 连接错误（mqtt broker 未运行）
- 不影响 WebSocket 功能
- 仅影响设备通信功能（本次测试不需要）

---

## 🎤 演示语音交互

### 测试场景 1：基础连接测试

**目的**：验证 WebSocket 连接和 Visitor ID 生成

**步骤：**

1. **打开开发者工具**
   - Windows/Linux: 按 `F12`
   - macOS: `Cmd + Option + I`
   - 切换到 **Console** 标签

2. **查看 Visitor ID**
   ```javascript
   // Console 输出示例：
   FingerprintJS Visitor ID: abc123def456...
   ```
   - 这是自动生成的浏览器指纹 ID
   - 同一浏览器/设备会保持一致

3. **建立 WebSocket 连接**
   - 点击页面上的 **"连接"** 按钮
   - 默认 URL：`ws://localhost:10031/ws/`

4. **验证连接成功**
   - ✅ 页面顶部显示：`已连接: abc123def456...`
   - ✅ 连接按钮变为红色 "断开"
   - ✅ 聊天区域显示：`WebSocket 连接成功`
   - ✅ Console 无错误信息

**故障排除：**
- ❌ 连接失败 → 检查 Bridge 服务是否运行（端口 10031）
- ❌ 无 Visitor ID → 检查网络连接（FingerprintJS CDN）

---

### 测试场景 2：对话模式（完整 AI 对话）⭐

**目的**：演示完整的语音识别 → AI 处理 → 语音合成流程

**前提条件：**
- ✅ EchoKit Server URL 已正确配置
- ✅ 麦克风权限已授予
- ✅ 网络连接正常

**步骤：**

#### 2.1 准备阶段

1. **确认模式设置**
   - ❌ "录制模式" 复选框**未勾选**（默认状态）
   - ✅ "显示调试信息" 可选勾选（建议勾选以查看详细流程）

2. **授权麦克风权限**
   - 首次使用时浏览器会弹出权限请求
   - 必须点击 **"允许"** 才能继续

3. **确认 VAD 初始化**
   - 聊天区域应显示：`✅ VAD 初始化完成`
   - 按钮文字显示：`开始监听`

#### 2.2 开始对话

1. **启动语音监听**
   - 方法 1：点击 **"开始监听"** 按钮
   - 方法 2：按键盘 **空格键**（推荐）

2. **观察状态变化**
   - 按钮变为红色
   - 文字变为 "停止监听"
   - 聊天区域显示：`🎧 VAD 开始监听 (对话模式)`

#### 2.3 语音输入

1. **对着麦克风说话**
   - 示例："你好，今天天气怎么样？"
   - 建议：清晰发音，正常语速

2. **检测到语音时**
   - 🎤 显示：`检测到语音开始`
   - 按钮开始脉冲动画（跳动效果）
   - 状态显示：`正在录制...`

3. **停止说话后（自动）**
   - VAD 自动检测语音结束
   - 显示：`🔇 语音结束 - 采样点: xxxxx`
   - 停止脉冲动画
   - 音频自动发送到服务器

#### 2.4 处理流程

**发送阶段：**
```
用户说话 → VAD 检测 → 录制音频 → 发送到 Bridge
```
- 聊天区域右侧显示语音消息（可点击播放）
- Console 显示：`发送 VAD 音频数据到服务器，长度: xxxxx`

**处理阶段：**
```
Bridge → EchoKit Server → ASR 识别 → AI 处理 → TTS 合成
```
- 等待时间：通常 1-3 秒

**接收阶段：**
```
EchoKit → Bridge → 浏览器
```

#### 2.5 接收响应

**查看 ASR 识别结果（右侧）：**
```
🎤 你好，今天天气怎么样？
```
- 显示为用户消息（右侧气泡）
- 蓝色背景

**查看 AI 文本回复（左侧）：**
```
🔊: 你好！今天天气晴朗，气温适宜，非常适合外出活动。
```
- 显示为服务器消息（左侧气泡）
- 灰色背景

**播放 AI 语音回复：**
- 自动播放 TTS 合成语音
- 可点击 "语音回复" 按钮重新播放
- 音频格式：16kHz, 16-bit PCM WAV

#### 2.6 多轮对话

1. **等待语音播放完成**（重要！）
2. **继续说话**
   - 例如："那明天呢？"
   - VAD 自动检测并发送

3. **观察上下文连续性**
   - AI 会记住之前的对话内容
   - 可以进行自然的多轮交互

4. **结束对话**
   - 点击 "停止监听" 按钮
   - 或按空格键关闭监听

---

### 测试场景 3：录制模式（仅音频收集）

**目的**：测试音频采集功能，不进行 AI 处理

**使用场景：**
- 🔧 调试音频质量
- 📊 收集语音数据集
- 💰 节省 EchoKit API 调用次数

**步骤：**

#### 3.1 切换到录制模式

1. **勾选 "录制模式" 复选框**
   - 聊天区域显示：`🔄 切换到录制模式`

2. **观察 WebSocket URL 变化**
   - 自动添加查询参数：`?record=true`
   - 例如：`ws://localhost:10031/ws/abc123?record=true`

#### 3.2 开始录制

1. **点击 "开始监听"**
   - 如果已连接，会自动重新连接（添加 record 参数）
   - 显示：`🔄 录制模式：重新连接服务器...`

2. **确认模式激活**
   - 显示：`🎧 VAD 开始监听 (录制模式)`

#### 3.3 录制音频

1. **对着麦克风说话**
   - VAD 检测流程与对话模式相同
   - 音频会发送到 Bridge

2. **观察差异**
   - ✅ 发送音频数据
   - ❌ **不会**转发到 EchoKit
   - ❌ **不会**收到 ASR 识别结果
   - ❌ **不会**收到 AI 回复
   - ❌ **不会**收到 TTS 语音

3. **查看 Bridge 日志**
   ```bash
   tail -f logs/bridge.log
   ```
   - 应显示：`Client visitor_xxx connecting (record_mode: true)`
   - 音频数据被接收但不处理

#### 3.4 用途

**调试音频质量：**
- 检查麦克风音量
- 验证 VAD 灵敏度
- 测试采样率和格式

**收集数据集：**
- Bridge 可以记录原始音频
- 用于训练或分析
- 不消耗 EchoKit API 配额

---

## 🔍 调试技巧

### 1. 使用调试模式

**启用方法：**
- 勾选 **"显示调试信息"** 复选框

**显示内容：**

**MessagePack 消息（左侧）：**
```
📦 MessagePack 数据: {
  "ASR": ["你好"]
}
```

**JSON 消息（左侧）：**
```
📄 JSON 数据: {
  "event": "StartChat"
}
```

**音频块信息：**
```
🔊 音频数据块 (2048 字节)
```

**发送信息：**
```
📤 发送结束标识: Normal
```

---

### 2. 使用浏览器开发者工具

#### Console 标签

**查看关键信息：**
```javascript
// Visitor ID
FingerprintJS Visitor ID: abc123def456...

// 音频处理日志
发送 VAD 音频数据到服务器，长度: 48000
发送结束标识: Normal

// 音频合成日志
处理完成的音频: 15 个块, 总长度: 30720 字节, 时长: 0.96 秒
```

**检查错误：**
- 红色文字表示错误
- 查看完整错误堆栈

#### Network → WS 标签

**查看 WebSocket 消息：**

1. **切换到 WS 标签**
2. **选择 WebSocket 连接**
   - 名称：通常以 `ws` 开头

3. **查看消息类型：**
   - 🟢 **绿色**：JSON 文本消息
     ```json
     {"event": "StartChat"}
     ```
   - 🟣 **粉色/紫色**：MessagePack 二进制消息
     ```
     Binary Message (Length: 2048)
     ```

4. **查看消息内容：**
   - 点击消息查看详细数据
   - 查看发送/接收时间戳

#### Application 标签

**查看 Storage：**
- LocalStorage：可能存储设置
- SessionStorage：会话数据
- Cookies：可能包含认证信息

---

### 3. 查看 Bridge 服务日志

**实时查看日志：**
```bash
tail -f /Volumes/Dev/secondstate/me/etch/bridge/logs/bridge.log
```

**关键日志信息：**

**WebSocket 连接：**
```
INFO Device visitor_abc123 WebSocket connected
INFO Client visitor_abc123 connecting (record_mode: false)
```

**音频处理：**
```
INFO Received audio data: 48000 samples
INFO Forwarding to EchoKit...
```

**EchoKit 交互：**
```
INFO EchoKit session created: session_xyz789
INFO Received ASR result: "你好"
INFO Received TTS audio chunks: 15
```

**错误信息：**
```
ERROR EchoKit connection failed: Connection refused
WARN Audio processing timeout
```

---

### 4. 常见问题排查

#### 问题 1：麦克风无权限

**现象：**
- 点击 "开始监听" 无反应
- Console 显示权限错误

**解决方案：**
1. 检查浏览器地址栏左侧的🔒图标
2. 点击 → 网站设置 → 麦克风
3. 选择 "允许"
4. 刷新页面

#### 问题 2：没有声音输出

**现象：**
- 收到文本回复但无语音
- Console 显示音频播放失败

**排查步骤：**
1. **检查系统音量**
   - 确保音量未静音
   - 音量设置合适

2. **检查浏览器音量**
   - 右键点击标签页
   - 查看是否静音

3. **查看 Console 错误**
   ```javascript
   音频播放失败: NotAllowedError
   ```
   - 可能需要用户交互才能播放
   - 点击页面任意位置后重试

4. **检查音频数据**
   - 确认收到 AudioChunk 消息
   - 查看音频块数量和大小

#### 问题 3：WebSocket 连接失败

**现象：**
- 点击 "连接" 后显示连接失败
- 状态显示 "连接已断开"

**排查步骤：**
1. **确认 Bridge 服务运行**
   ```bash
   lsof -i :10031
   ```
   - 应该显示 echo-bridge 进程

2. **检查 WebSocket URL**
   - 确认为 `ws://localhost:10031/ws/`
   - 注意协议：本地用 `ws://`，生产用 `wss://`

3. **查看 Bridge 日志**
   ```bash
   tail -f logs/bridge.log
   ```
   - 查看是否有连接尝试日志
   - **忽略 MQTT 错误**（不影响 WebSocket）

4. **检查防火墙**
   - 确保端口 10031 未被阻止

#### 问题 3.5：看到 MQTT 错误

**现象：**
```
ERROR echo_bridge::mqtt_client: MQTT connection error
ERROR echo_bridge::mqtt_client: MQTT event loop terminated with error
```

**解决方案：**
- ✅ **这是正常的！可以安全忽略**
- MQTT broker 未运行（本地测试不需要）
- 不影响 WebSocket 和对话功能
- 只要看到 `WebSocket server listening on: 0.0.0.0:10031` 就表示服务正常

#### 问题 4：EchoKit 连接超时

**现象：**
- 发送音频后长时间无响应
- Console 显示超时错误

**排查步骤：**
1. **检查网络连接**
   ```bash
   ping indie.echokit.dev
   ```

2. **验证 EchoKit URL 格式**
   - 正确格式：`wss://indie.echokit.dev/ws/{visitor-id}`
   - 检查 visitor-id 是否有效

3. **查看 Bridge 日志**
   ```bash
   grep -i "echokit" logs/bridge.log
   ```
   - 查看连接状态和错误信息

4. **测试 EchoKit 服务**
   - 访问 https://indie.echokit.dev
   - 确认服务正常

#### 问题 5：VAD 检测不灵敏

**现象：**
- 说话后没有检测到语音
- 或过于灵敏（误触发）

**调整方法：**
1. **检查麦克风音量**
   - 系统设置 → 声音 → 输入
   - 调整输入音量

2. **靠近/远离麦克风**
   - 测试不同距离
   - 找到最佳距离

3. **环境噪音**
   - 在安静环境测试
   - 减少背景噪音

4. **更换麦克风**
   - 尝试不同的麦克风设备
   - 使用外置麦克风效果更好

---

## 📱 快捷键

| 快捷键 | 功能 | 说明 |
|--------|------|------|
| `空格` | 开始/停止监听 | 页面未聚焦输入框时有效 |
| `F12` | 开发者工具 | Windows/Linux |
| `Cmd+Option+I` | 开发者工具 | macOS |
| `Ctrl+C` | 停止服务 | 在终端中 |

---

## 🎬 完整演示流程示例

### 场景：首次完整演示

```bash
# === 终端操作 ===
cd /Volumes/Dev/secondstate/me/etch/bridge
./start_test.sh

# 首先显示服务依赖清单
# 说明需要哪些服务和端口

# 交互提示：
# - 编译？y
# - UDP 端口？（如有冲突则输入替代端口）
# - EchoKit URL？（回车使用默认）
# - 打开浏览器？y

# ⚠️ 可能看到 MQTT 错误（可忽略）
# ✅ 关键日志：WebSocket server listening on: 0.0.0.0:10031

# === 浏览器操作 ===
# 1. 浏览器自动打开 http://localhost:10031/bridge_webui.html
# 2. 按 F12 打开开发者工具
# 3. 切换到 Console 标签
# 4. 查看 Visitor ID 输出

# 5. 点击 "连接" 按钮
#    ✅ 显示：已连接: abc123...

# 6. 勾选 "显示调试信息"（可选）

# 7. 点击 "开始监听" 或按空格键
#    ✅ 显示：🎧 VAD 开始监听 (对话模式)

# 8. 对着麦克风说："你好"
#    ✅ 显示：🎤 检测到语音开始
#    ✅ 按钮脉冲动画

# 9. 停止说话（自动检测结束）
#    ✅ 音频自动发送

# 10. 等待响应（1-3秒）
#     ✅ 右侧显示：🎤 你好
#     ✅ 左侧显示：🔊: 你好！很高兴见到你
#     ✅ 自动播放语音回复

# 11. 继续对话
#     再次说话："今天天气怎么样？"
#     重复步骤 8-10

# 12. 测试录制模式
#     - 勾选 "录制模式"
#     - 点击 "开始监听"
#     - 说话测试
#     - 观察只发送不处理

# 13. 完成演示
#     - 点击 "停止监听"
#     - 查看完整对话历史

# === 停止服务 ===
# 在另一个终端运行：
cd /Volumes/Dev/secondstate/me/etch/bridge
./stop_test.sh

# 或在运行 Bridge 的终端按 Ctrl+C
```

---

## 🎯 测试检查清单

### 启动阶段
- [ ] Bridge 服务启动成功（端口 10031）
- [ ] 浏览器成功打开测试页面
- [ ] 页面显示 Visitor ID（Console）
- [ ] 静态文件正常加载（无 404 错误）

### 连接阶段
- [ ] WebSocket 连接成功
- [ ] 页面显示连接状态（绿色）
- [ ] Bridge 日志显示连接记录

### VAD 阶段
- [ ] VAD 初始化完成
- [ ] 能检测到语音开始
- [ ] 能检测到语音结束
- [ ] 音频数据成功录制

### 对话模式测试
- [ ] 音频成功发送到服务器
- [ ] 收到 ASR 识别结果（右侧）
- [ ] 收到 AI 文本回复（左侧）
- [ ] 收到并播放 TTS 语音
- [ ] 可以进行多轮对话
- [ ] 上下文连续性正常

### 录制模式测试
- [ ] WebSocket URL 包含 record=true
- [ ] 音频正常发送
- [ ] 不调用 EchoKit（节省 API）
- [ ] Bridge 日志显示录制模式
- [ ] 音频数据被正确处理

### 调试功能
- [ ] 显示调试信息正常工作
- [ ] Console 显示详细日志
- [ ] Network/WS 标签显示消息
- [ ] Bridge 日志文件正常记录

---

## 📊 性能指标参考

### 正常响应时间

| 阶段 | 预期时间 | 说明 |
|------|---------|------|
| WebSocket 连接 | < 100ms | 本地连接应该很快 |
| VAD 检测语音开始 | < 200ms | 实时检测 |
| VAD 检测语音结束 | 500-1500ms | 根据停顿时间 |
| 音频上传 | < 500ms | 取决于音频长度 |
| ASR 识别 | 500-2000ms | EchoKit 处理时间 |
| AI 生成回复 | 1000-3000ms | 取决于回复长度 |
| TTS 合成 | 500-1500ms | EchoKit 处理时间 |
| 音频播放开始 | < 100ms | 本地播放 |
| **总端到端延迟** | **2-6秒** | 从说话到听到回复 |

### 音频规格

| 参数 | 值 | 说明 |
|------|-----|------|
| 采样率 | 16000 Hz | 16 kHz |
| 位深度 | 16 bit | Int16 PCM |
| 声道 | 1 (Mono) | 单声道 |
| 编码格式 | PCM | 未压缩 |
| 传输格式 | WAV | 带 44 字节头 |
| 数据块大小 | ~2048 字节 | 可变 |

---

## 🔄 停止服务

### 使用停止脚本

```bash
cd /Volumes/Dev/secondstate/me/etch/bridge
./stop_test.sh
```

**输出示例：**
```
🛑 停止 Bridge 测试服务...
🔴 停止 Bridge 服务 (PID: 12345)...
✅ Bridge 服务已停止
✅ 测试服务已停止
📋 日志文件保留在 logs/ 目录中
```

### 手动停止

**如果使用自动化脚本启动：**
```bash
# 查找 PID
cat logs/bridge.pid

# 停止进程
kill $(cat logs/bridge.pid)
```

**如果使用 cargo run：**
- 在运行终端按 `Ctrl+C`

**强制停止：**
```bash
# 查找端口占用
lsof -i :10031

# 强制杀死进程
kill -9 <PID>
```

---

## 📚 相关文档

- **TESTING_GUIDE.md** - 完整测试指南（包含所有测试场景）
- **WEBUI_INTEGRATION_TASKS.md** - 技术实现细节
- **README.md** - 项目总体说明
- **docker-compose.yml** - Docker 部署配置

---

## 💡 提示与技巧

### 最佳实践

1. **首次使用前**
   - 运行完整编译：`cargo build --release`
   - 检查所有依赖项已安装
   - 在安静环境测试

2. **演示准备**
   - 提前启动服务（避免等待编译）
   - 测试麦克风和扬声器
   - 准备好示例对话内容

3. **多人演示**
   - 使用外置麦克风（音质更好）
   - 调整音量到合适水平
   - 考虑使用屏幕共享

4. **调试技巧**
   - 始终开启调试信息复选框
   - 保持开发者工具打开
   - 实时查看 Bridge 日志

### 高级功能

**自定义 Visitor ID：**
```javascript
// 在浏览器 Console 中
window.visitorId = "my-custom-id";
location.reload();
```

**修改 WebSocket URL：**
- 直接在输入框中编辑
- 支持 ws:// 和 wss:// 协议
- 可以连接到远程 Bridge 服务

**清除聊天历史：**
- 点击 "清除聊天" 按钮
- 或刷新页面

---

## 🆘 获取帮助

**查看日志：**
```bash
# Bridge 日志
tail -f logs/bridge.log

# 系统日志（macOS）
log show --predicate 'process == "echo-bridge"' --last 1m
```

**检查服务状态：**
```bash
# 检查端口
lsof -i :10031

# 检查进程
ps aux | grep bridge
```

**重启服务：**
```bash
./stop_test.sh
./start_test.sh
```

---

## 📝 更新日志

- **2025-11-06**: 更新演示指南文档
  - 添加服务依赖清单说明
  - 添加 UDP 端口冲突处理说明
  - 说明 MQTT 错误可以安全忽略
  - 完整演示流程包含新增的交互步骤

- **2025-11-06**: 创建演示指南文档
  - 添加完整演示流程
  - 包含调试技巧和故障排除
  - 提供性能指标参考

---

**祝您演示顺利！** 🎉
