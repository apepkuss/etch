# 设备电量信息获取机制说明

## 问题

在【设备管理】页面的设备列表中，【电量】列显示的是设备的电量信息。用户提问：**这是实时电量吗？电量信息是如何获得的？**

## 当前实现分析

### 1. 数据库层面

#### devices 表结构

```sql
CREATE TABLE devices (
    id UUID PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    device_type VARCHAR(50),
    status VARCHAR(50),
    firmware_version VARCHAR(50),
    battery_level INTEGER,  -- 电量字段 (0-100)
    volume INTEGER,
    last_seen TIMESTAMP WITH TIME ZONE,
    is_online BOOLEAN,
    owner VARCHAR(255),
    ...
    CONSTRAINT devices_battery_level_check CHECK (battery_level >= 0 AND battery_level <= 100)
);
```

**关键发现**：
- `battery_level` 是整数类型，范围 0-100（百分比）
- 存储在数据库中，非实时计算值
- 查询当前所有设备：**所有设备的 `battery_level` 都是 0**

```bash
$ docker exec echo-postgres psql -U echo_user -d echo_db -c "SELECT id, name, battery_level FROM devices;"

id                               | name          | battery_level
---------------------------------+---------------+--------------
ECHO_ES20250101001_a1b2c3d4e5f6 | 测试音箱2      | 0
ECHO_TEST002_b2c3d4e5f6a1       | 测试智能音箱2  | 0
ECHO_NEW001_c1d2e3f4a5b6        | TestNewDevice | 0
```

### 2. 前端显示层面

#### DeviceList.tsx

```typescript
// echo-web-management/src/pages/DeviceList.tsx

{
  title: '电量',
  dataIndex: 'battery_level',
  key: 'battery_level',
  align: 'center',
  render: (level: number) => (
    <Progress
      percent={level}
      size="small"
      status={level < 20 ? 'exception' : level < 50 ? 'normal' : 'success'}
      strokeColor={level < 20 ? '#ff4d4f' : level < 50 ? '#faad14' : '#52c41a'}
    />
  )
}
```

**工作方式**：
- 直接从 `Device` 对象的 `battery_level` 字段读取
- 使用 Ant Design 的 `<Progress>` 组件显示进度条
- 颜色逻辑：
  - 红色 (exception): `< 20%`
  - 黄色 (normal): `20-50%`
  - 绿色 (success): `> 50%`

### 3. API 层面

#### 前端 API 调用

```typescript
// echo-web-management/src/api/devices.ts

async getDevices(): Promise<Device[]> {
  const response = await apiClient.get<ApiResponse<any>>('/devices');

  if (response.data.data && response.data.data.items) {
    return response.data.data.items;  // 返回设备列表
  }
  // ...
}
```

**API 端点**：`GET /devices`

**数据流**：
```
Frontend (DeviceList.tsx)
    ↓ fetchDevices()
useDeviceStore
    ↓ devicesApi.getDevices()
API Gateway (GET /devices)
    ↓ SQL Query
PostgreSQL (devices 表)
    ↓ 返回 battery_level
Frontend 显示进度条
```

### 4. 后端更新机制

#### API Gateway - DeviceService

```rust
// api-gateway/src/device_service.rs

pub async fn update_device_status(
    &self,
    device_id: &str,
    status: DeviceStatus,
    battery_level: Option<i32>,  // 可选的电量参数
    volume: Option<i32>,
    last_seen: Option<chrono::DateTime<chrono::Utc>>,
    is_online: Option<bool>,
) -> Result<bool> {
    sqlx::query!(
        r#"
        UPDATE devices
        SET status = $1,
            battery_level = COALESCE($2, battery_level),  -- 如果提供则更新
            volume = COALESCE($3, volume),
            last_seen = COALESCE($4, NOW()),
            is_online = COALESCE($5, is_online),
            updated_at = NOW()
        WHERE id = $6
        "#,
        status_str,
        battery_level,
        volume,
        last_seen,
        is_online,
        device_uuid
    )
    .execute(&self.pool)
    .await?;

    Ok(true)
}
```

**关键发现**：
- `battery_level` 是可选参数 (`Option<i32>`)
- 使用 `COALESCE($2, battery_level)` - 如果提供新值则更新，否则保持原值
- 更新时会同时更新 `updated_at` 时间戳

#### Bridge - MQTT 消息发布

```rust
// bridge/src/mqtt_client.rs

pub async fn publish_device_status(
    &self,
    device_id: &str,
    status: DeviceStatus,
    battery_level: Option<i32>,  // 设备电量
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
```

**电量更新流程**：
```
硬件设备
    ↓ 通过 MQTT 发送电量信息
Bridge (mqtt_client.rs)
    ↓ publish_device_status()
API Gateway (MQTT 订阅)
    ↓ update_device_status()
PostgreSQL (devices.battery_level)
    ↓ 数据更新
WebSocket 广播 (可选)
    ↓ DeviceStatusUpdate 消息
Frontend 自动刷新显示
```

### 5. WebSocket 实时更新机制

#### 前端 WebSocket 处理

```typescript
// echo-web-management/src/stores/useDeviceStore.ts

websocketService.connect({
  onMessage: (message) => {
    // 处理设备状态更新
    if (message.DeviceStatusUpdate) {
      const { device_id, status } = message.DeviceStatusUpdate;
      useDeviceStore.getState().updateDeviceStatus(device_id, status);
    }
  }
});
```

**当前问题**：
- ❌ WebSocket 消息类型 `DeviceStatusUpdate` **不包含 `battery_level` 字段**
- ❌ 只更新 `status` 和 `is_online`，不更新电量
- ✅ 电量变化不会通过 WebSocket 实时推送到前端

#### WebSocket 消息定义

```typescript
// echo-web-management/src/types/index.ts

export interface WebSocketMessage {
  DeviceStatusUpdate?: {
    device_id: string;
    status: DeviceStatus;
    timestamp: string;
    // ❌ 缺少 battery_level 字段
  };
}

export const MessageType = {
  DEVICE_STATUS_CHANGE: 'device_status_change',
  DEVICE_BATTERY_UPDATE: 'device_battery_update',  // ⚠️ 定义了但未使用
  SESSION_STARTED: 'session_started',
  SESSION_UPDATED: 'session_updated',
  DEVICE_ERROR: 'device_error'
} as const;
```

**发现**：
- 类型定义中有 `DEVICE_BATTERY_UPDATE`，但**没有被实现或使用**
- 电量更新消息类型已定义但未连接到实际功能

---

## 回答用户问题

### ❌ 不是实时电量

**原因**：

1. **数据来源是数据库快照**：
   - 前端显示的电量来自数据库的 `devices.battery_level` 字段
   - 不是从设备直接读取的实时值

2. **缺少实时更新机制**：
   - WebSocket 不推送电量变化
   - 前端不会自动更新电量显示
   - 只有手动刷新页面或定时轮询才能看到新值

3. **当前所有设备电量为 0**：
   - 测试数据中所有设备的 `battery_level` 都是 0
   - 说明**从未收到过设备电量更新**

### 电量信息获取方式

#### 理论设计流程（应该如何工作）

```
1. 硬件设备定期上报电量
   └─> 通过 MQTT 消息发送到 Bridge

2. Bridge 接收并转发
   └─> 调用 API Gateway 的 update_device_status()

3. API Gateway 更新数据库
   └─> UPDATE devices SET battery_level = $1 WHERE id = $2

4. API Gateway 广播 WebSocket 消息（理想情况）
   └─> DeviceBatteryUpdate { device_id, battery_level, timestamp }

5. 前端接收 WebSocket 消息
   └─> 自动更新 UI 显示新电量

6. 或者，前端定期调用 GET /devices
   └─> 获取最新的设备列表（包含最新电量）
```

#### 实际工作流程（当前实现）

```
1. ✅ 硬件设备通过 MQTT 发送电量
   └─> Bridge 的 publish_device_status() 支持 battery_level 参数

2. ✅ Bridge 可以接收并转发
   └─> API Gateway 的 update_device_status() 支持更新 battery_level

3. ✅ API Gateway 可以更新数据库
   └─> SQL: battery_level = COALESCE($2, battery_level)

4. ❌ WebSocket 不推送电量更新
   └─> DeviceBatteryUpdate 消息类型未实现

5. ❌ 前端不会自动更新电量
   └─> 只有手动刷新页面才能看到新值

6. ✅ 用户手动点击"刷新"按钮
   └─> 调用 fetchDevices() → GET /devices → 显示最新电量
```

---

## 问题总结

### 核心问题

1. **电量不是实时的**：
   - 显示的是数据库中的历史值
   - 需要手动刷新才能看到最新值

2. **缺少实时更新机制**：
   - WebSocket 没有推送电量变化
   - 前端没有自动刷新逻辑

3. **测试数据不完整**：
   - 所有设备的 `battery_level` 都是 0
   - 可能是设备从未上报电量，或上报逻辑未实现

### 设计已具备的能力

✅ **基础设施完善**：
- 数据库有 `battery_level` 字段 (0-100)
- API Gateway 支持更新电量
- Bridge MQTT 支持发送电量
- 前端 UI 支持显示电量进度条

❌ **实时性缺失**：
- 无 WebSocket 电量推送
- 无自动刷新机制
- 无设备端上报实现（或未启用）

---

## 改进建议

### 短期方案：定时轮询

在前端添加定时刷新逻辑：

```typescript
// echo-web-management/src/pages/DeviceList.tsx

useEffect(() => {
  // 每 30 秒刷新一次设备列表
  const interval = setInterval(() => {
    fetchDevices();
  }, 30000);

  return () => clearInterval(interval);
}, [fetchDevices]);
```

**优点**：
- 实现简单
- 无需修改后端

**缺点**：
- 不是真正的实时
- 增加服务器负担（频繁轮询）
- 30秒延迟

### 长期方案：WebSocket 实时推送

#### Step 1：扩展 WebSocket 消息类型

```typescript
// echo-web-management/src/types/index.ts

export interface WebSocketMessage {
  DeviceStatusUpdate?: {
    device_id: string;
    status: DeviceStatus;
    timestamp: string;
  };

  // 新增：电量更新消息
  DeviceBatteryUpdate?: {
    device_id: string;
    battery_level: number;
    timestamp: string;
  };
}
```

#### Step 2：后端广播电量更新

```rust
// api-gateway/src/device_service.rs

pub async fn update_device_status(...) -> Result<bool> {
    // 更新数据库
    sqlx::query!(...).execute(&self.pool).await?;

    // 广播 WebSocket 消息
    if let Some(new_battery) = battery_level {
        let message = WebSocketMessage::DeviceBatteryUpdate {
            device_id: device_id.to_string(),
            battery_level: new_battery,
            timestamp: chrono::Utc::now(),
        };
        websocket_broadcast(message).await?;
    }

    Ok(true)
}
```

#### Step 3：前端处理电量更新

```typescript
// echo-web-management/src/stores/useDeviceStore.ts

websocketService.connect({
  onMessage: (message) => {
    // 处理设备状态更新
    if (message.DeviceStatusUpdate) {
      const { device_id, status } = message.DeviceStatusUpdate;
      useDeviceStore.getState().updateDeviceStatus(device_id, status);
    }

    // 新增：处理电量更新
    if (message.DeviceBatteryUpdate) {
      const { device_id, battery_level } = message.DeviceBatteryUpdate;
      useDeviceStore.getState().updateDeviceBattery(device_id, battery_level);
    }
  }
});

// 新增方法
updateDeviceBattery: (deviceId: string, batteryLevel: number) => {
  set(state => ({
    devices: state.devices.map(device =>
      device.id === deviceId
        ? { ...device, battery_level: batteryLevel }
        : device
    )
  }));
}
```

#### Step 4：设备端定期上报电量

```
硬件设备需要实现：
1. 读取电池电量（如果有电池）
2. 每 5 分钟通过 MQTT 发送电量更新
3. 电量变化超过 10% 时立即发送
```

**优点**：
- 真正的实时更新
- 高效（仅在变化时推送）
- 用户体验好

**缺点**：
- 需要修改前后端代码
- 需要设备端支持上报
- 实现复杂度较高

---

## 结论

### 当前状态 ⚠️

Echo System 的设备电量信息**不是实时的**：

1. **显示来源**：数据库中的历史值（`devices.battery_level`）
2. **更新方式**：手动刷新页面时重新查询数据库
3. **当前值**：所有设备都是 0%（未收到过电量更新）

### 设计能力 ✅

系统**已具备电量管理的基础能力**：

- ✅ 数据库支持存储电量 (0-100)
- ✅ API 支持更新电量
- ✅ MQTT 支持传输电量
- ✅ 前端 UI 支持显示电量

### 缺少部分 ❌

**缺少实时性实现**：

- ❌ 无 WebSocket 电量推送机制
- ❌ 无前端自动刷新逻辑
- ❌ 设备端未上报电量（或未启用）

### 推荐方案 🎯

**阶段 1（临时）**：添加定时轮询（30秒刷新一次）
**阶段 2（最终）**：实现 WebSocket 实时推送 + 设备定期上报
