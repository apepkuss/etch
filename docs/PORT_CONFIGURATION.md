# Echo System 端口配置文档

## 概述

本文档记录了 Echo System 项目的端口配置信息。为了系统化管理和避免端口冲突，所有服务的外部访问端口已统一配置为以 `10030` 开始的端口范围。

## 端口分配方案

### 服务端口映射表

| 服务名称 | 外部端口 | 内部端口 | 协议 | 说明 |
|---------|----------|----------|------|------|
| **EchoKit Server** | **10030** | 9988 | HTTP/WS | AI 推理服务 |
| **Bridge WebSocket** | **10031** | 8082 | WebSocket | Bridge WebSocket 服务 |
| **Bridge UDP** | **10032** | 8083 | UDP | Bridge UDP 通信 |
| **API Gateway** | **10033** | 8080 | HTTP/WS | 主要 API 服务 |
| **Web Management** | **10034** | 5174 | HTTP | React 管理界面 |
| **PostgreSQL** | **10035** | 5432 | TCP | 主数据库 |
| **Redis** | **10036** | 6379 | TCP | 缓存服务 |
| **pgAdmin** | **10037** | 80 | HTTP | 数据库管理界面 |
| **Redis Commander** | **10038** | 8081 | HTTP | Redis 管理界面 |
| **MQTT Broker** | **10039** | 1883 | TCP | MQTT 消息队列 |
| **MQTT WebSocket** | **10040** | 9001 | WebSocket | MQTT WebSocket |

### 服务间通信配置（内部网络）

服务间通信使用 Docker 内部网络，配置保持不变：

| 源服务 | 目标服务 | 连接地址 | 说明 |
|--------|----------|----------|------|
| Bridge | EchoKit Server | `ws://echokit-server:9988/v1/realtime` | WebSocket 连接 |
| Bridge | PostgreSQL | `postgres:5432` | 数据库连接 |
| Bridge | Redis | `redis:6379` | 缓存连接 |
| API Gateway | Bridge | `ws://bridge:8082` | WebSocket 连接 |
| 各服务 | MQTT Broker | `tcp://mqtt:1883` | MQTT 连接 |

## 配置文件修改记录

### 1. docker-compose.yml
```yaml
# 主要修改的端口映射
ports:
  - "10030:9988"   # EchoKit Server
  - "10031:8082"   # Bridge WebSocket
  - "10032:8083"   # Bridge UDP
  - "10033:8080"   # API Gateway
  - "10034:5174"   # Web Management
  - "10035:5432"   # PostgreSQL
  - "10036:6379"   # Redis
  - "10037:80"     # pgAdmin
  - "10038:8081"   # Redis Commander
  - "10039:1883"   # MQTT Broker
  - "10040:9001"   # MQTT WebSocket

# 环境变量更新
environment:
  REACT_APP_API_BASE_URL: http://localhost:10033
  REACT_APP_WS_URL: ws://localhost:10033
  CORS_ORIGINS: "http://localhost:10034,http://localhost:3000"
```

### 2. .env.example
```bash
# 数据库和缓存配置
DATABASE_URL=postgres://echo_user:echo_password@localhost:10035/echo_db
REDIS_URL=redis://:redis_password@localhost:10036

# EchoKit Server 配置
ECHOKIT_WEBSOCKET_URL=ws://localhost:10030/v1/realtime
ECHOKET_API_BASE_URL=http://localhost:10030

# MQTT 配置
MQTT_BROKER_URL=tcp://localhost:10039

# 服务端口配置
API_GATEWAY_PORT=10033
WEBSOCKET_PORT=10031
UDP_PORT=10032
WEB_MANAGEMENT_PORT=10034
POSTGRES_PORT=10035
REDIS_PORT=10036
ECHOKIT_SERVER_PORT=10030
PGADMIN_PORT=10037
REDIS_COMMANDER_PORT=10038
MQTT_PORT=10039
MQTT_WS_PORT=10040

# 前端配置
REACT_APP_API_BASE_URL=http://localhost:10033
REACT_APP_WS_URL=ws://localhost:10033
CORS_ORIGINS=http://localhost:10034,http://localhost:3000
```

### 3. api-gateway/.env.example
```bash
# 服务器配置
APP_SERVER_PORT=8080  # 内部端口，保持不变

# 数据库配置
APP_DATABASE_URL=postgresql://echo_user:echo_pass@localhost:10035/echo_db

# Redis 配置
APP_REDIS_URL=redis://localhost:10036

# MQTT 配置
APP_MQTT_PORT=10039
```

## 使用指南

### 启动服务
```bash
# 使用默认配置启动所有服务
docker-compose up -d

# 检查服务状态
docker-compose ps
```

### 访问服务

| 服务 | 访问地址 | 说明 |
|------|----------|------|
| API Gateway | http://localhost:10033 | 主 API 服务 |
| Web Management | http://localhost:10034 | React 管理界面 |
| pgAdmin | http://localhost:10037 | 数据库管理 |
| Redis Commander | http://localhost:10038 | Redis 管理 |
| EchoKit Server | http://localhost:10030 | AI 推理服务 |

### 环境变量配置
```bash
# 复制环境变量模板
cp .env.example .env

# 根据需要修改端口配置
vim .env
```

## 重要说明

### 🔒 安全注意事项
1. **生产环境**：建议修改默认密码和 JWT 密钥
2. **防火墙**：确保端口 10030-10040 在防火墙中正确配置
3. **网络隔离**：生产环境建议只暴露必要的端口

### 🔄 内部 vs 外部端口
- **内部端口**：容器内服务监听的端口，保持不变
- **外部端口**：主机上映射的端口，已修改为 10030+ 范围
- **服务间通信**：使用 Docker 内部网络，通过服务名称和内部端口通信

### 🚀 端口范围规划
- **10030-10034**：核心业务服务
- **10035-10038**：数据和管理服务
- **10039-10040**：消息队列服务

### 🐛 故障排查
```bash
# 检查端口占用
netstat -tulpn | grep 1003

# 查看服务日志
docker-compose logs <service-name>

# 测试服务连通性
curl http://localhost:10030/health
curl http://localhost:10033/health
```
